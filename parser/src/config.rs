use nom::bytes::complete::{is_not, take_while, take_while1};
use nom::character::complete::{char, line_ending, space0};
use nom::combinator::{all_consuming, cut, eof, map, map_res, opt, peek, verify};
use nom::error::{ErrorKind, FromExternalError, ParseError};
use nom::multi::{many0, separated_list0, separated_list1};
use nom::sequence::{delimited, pair, preceded, separated_pair, terminated};
use nom::{IResult, Parser};
use thiserror::Error;

use crate::config::ConfigError::*;
use crate::module::parse_multiline_module;

/// This function optionally parses a `[config]` module, and if a `[config]` module is detected,
/// it will forcefully be parsed. If parsing fails, this parser fails with an appropriate
/// `ConfigError`
pub fn parse_config_module(input: &str) -> IResult<&str, Option<Config>, ConfigError> {
    // Okay, this is kind of complicated since we want our result to have the same lifetime
    // as the input, even though we have an IR of Module which doesn't have a lifetime
    // First: try to parse the "config" module
    let module = verify(
        preceded(many0(line_ending), parse_multiline_module),
        |module| module.name.to_ascii_lowercase().as_str() == "config",
    )(input);

    // Check if it was successful
    match module {
        // If we have no config module, return None and keep all our input
        Err(_) => Ok((input, None)),
        // If we do have a config module, "rest" is going to contain the rest of our input file to
        // be parsed, and "module" is our parsed module
        Ok((rest, module)) => {
            // Let's cut our config body (make sure it actually parses and otherwise fail the parse
            // altogether), and on successful parse, return our "rest" to be continued at parsing
            all_consuming(map(cut(parse_config_body), Some))(&module.body)
                .map(|(_, cfg)| (rest, cfg))
        }
    }
}

fn parse_config_body(input: &str) -> IResult<&str, Config, ConfigError> {
    // Our config body contains of newline-separated import statements
    map_res(
        separated_list1(line_ending, cut(terminated(config_statement, space0))),
        |imports| {
            imports
                .into_iter()
                .flatten()
                .try_fold(Config::default(), Config::try_append)
        },
    )(input)
}

fn config_statement(input: &str) -> IResult<&str, Option<ConfigAppendable>, ConfigError> {
    // Check if it is empty, and in that case, return it
    let empty_result = map(pair(space0, peek(line_ending.or(eof))), |_| None)(input);
    if empty_result.is_ok() {
        return empty_result;
    }

    // Get the first keyword
    let (rest, keyword) = delimited(space0, is_not(" \n\r"), space0)(input)
        .map_err(|e| e.map(|_e: nom::error::Error<&str>| InvalidConfigStatement))?;
    match keyword {
        "import" => map(import_statement, Into::into)(rest),
        "hide" => map(hide_statement, Into::into)(rest),
        "set" => map(set_statement, Into::into)(rest),
        _ => Err(nom::Err::Error(InvalidConfigKeyword(keyword.to_string()))),
    }
    .map(|(a, b)| (a, Some(b)))
}

fn set_statement(input: &str) -> IResult<&str, Set, ConfigError> {
    map(
        separated_pair(
            take_while(|c: char| !c.is_whitespace()),
            space0,
            is_not("\n\r"),
        ),
        |(key, value): (&str, &str)| {
            if value.starts_with('"') && value.ends_with('"') {
                let sub_value = &value[1..value.len() - 1];
                Set {
                    key: key.to_string(),
                    value: sub_value.to_string(),
                }
            } else {
                Set {
                    key: key.to_string(),
                    value: value.to_string(),
                }
            }
        },
    )(input)
    .map_err(|e| {
        e.map(|_e: nom::error::Error<&str>| {
            InvalidSetStatement(
                input
                    .lines()
                    .next()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            )
        })
    })
}

fn hide_statement(input: &str) -> IResult<&str, Hide, ConfigError> {
    map(take_while(|c: char| !c.is_whitespace()), |name: &str| {
        Hide {
            name: name.to_string(),
            hiding: HideConfig::HideAll,
        }
    })(input)
}

fn import_statement(input: &str) -> IResult<&str, Import, ConfigError> {
    map(
        pair(take_while(|c: char| !c.is_whitespace()), import_config),
        |(name, config)| Import {
            name: name.to_string(),
            importing: config,
        },
    )(input)
}

fn import_config(input: &str) -> IResult<&str, ImportConfig, ConfigError> {
    // three cases: either we have "using abc, def...", or "hiding abc, def...", or nothing
    map_res(
        opt(pair(
            delimited(space0, take_while1(|c: char| !c.is_whitespace()), space0),
            separated_list0(
                pair(char(','), space0),
                take_while1(|c: char| !c.is_whitespace() && c != ','),
            ),
        )),
        |res: Option<(&str, Vec<&str>)>| {
            if let Some((option, imports)) = res {
                if imports.is_empty() {
                    return Err(NoExclusionsSpecified(option.to_string()));
                }

                let map = imports.into_iter().map(&str::to_string).collect();
                let lowercase = option.to_lowercase();
                match lowercase.as_str() {
                    "using" => Ok(ImportConfig::Include(map)),
                    "hiding" => Ok(ImportConfig::Exclude(map)),
                    x => Err(InvalidImportSpecifier(x.to_string())),
                }
            } else {
                Ok(ImportConfig::ImportAll)
            }
        },
    )(input)
}

#[derive(Debug, Clone, Default, Hash)]
pub struct Config {
    pub imports: Vec<Import>,
    pub hides: Vec<Hide>,
    pub sets: Vec<Set>,
}

#[derive(Debug, Clone, Hash)]
pub struct Import {
    pub name: String,
    pub importing: ImportConfig,
}

#[derive(Debug, Clone, Hash)]
pub enum ImportConfig {
    ImportAll,
    Include(Vec<String>),
    Exclude(Vec<String>),
}

#[derive(Debug, Clone, Hash)]
pub struct Hide {
    pub name: String,
    pub hiding: HideConfig,
}

#[derive(Debug, Clone, Hash)]
pub struct Set {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Hash)]
pub enum HideConfig {
    HideAll,
}

enum ConfigAppendable {
    Import(Import),
    Hide(Hide),
    Set(Set),
}

impl Config {
    fn try_append(mut self, other: ConfigAppendable) -> Result<Self, ConfigError> {
        // Fixme: Check that there are no collisions here
        match other {
            ConfigAppendable::Import(i) => {
                self.imports.push(i);
            }
            ConfigAppendable::Hide(h) => {
                self.hides.push(h);
            }
            ConfigAppendable::Set(s) => {
                self.sets.push(s);
            }
        }
        Ok(self)
    }
}

impl From<Import> for ConfigAppendable {
    fn from(value: Import) -> Self {
        ConfigAppendable::Import(value)
    }
}

impl From<Hide> for ConfigAppendable {
    fn from(value: Hide) -> Self {
        ConfigAppendable::Hide(value)
    }
}

impl From<Set> for ConfigAppendable {
    fn from(value: Set) -> Self {
        ConfigAppendable::Set(value)
    }
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ConfigError {
    #[error("Invalid configuration statement: start with a keyword and then give options, like 'import foo'")]
    InvalidConfigStatement,
    // If we get an invalid keyword (the first word of a config line)
    #[error("Invalid configuration keyword '{0}', expected 'import', 'hide' or 'set'")]
    InvalidConfigKeyword(String),
    #[error("Invalid import statement '{0}', expected 'import package_name'")]
    InvalidImportStatement(String),
    #[error("Invalid import specifier '{0}', expected 'using' or 'hiding'")]
    InvalidImportSpecifier(String),
    #[error("Invalid set statement, expected 'set key value', got '{0}'")]
    InvalidSetStatement(String),
    #[error("'{0}' specified for an import but no transformations named")]
    NoExclusionsSpecified(String),
    #[error("Accumulation error")]
    AccumulationError,
    #[error("Unknown nom error when parsing config: '{0}', kind: '{1:?}'")]
    UnknownNomError(String, ErrorKind),
}

impl ParseError<&str> for ConfigError {
    fn from_error_kind(input: &str, kind: ErrorKind) -> Self {
        UnknownNomError(input.to_string(), kind)
    }

    fn append(_input: &str, _kind: ErrorKind, other: Self) -> Self {
        other
    }
}

impl FromExternalError<&str, ConfigError> for ConfigError {
    fn from_external_error(_input: &str, _kind: ErrorKind, e: ConfigError) -> Self {
        e
    }
}
