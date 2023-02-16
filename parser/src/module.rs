//! This module provides the function the Parser needs to parse modules. It exposes two functions;
//! [parse_inline_module] and [parse_multiline_module], which parses inline modules and multiline
//! modules respectively.
use crate::{MaybeArgs, Module, ModuleArguments, ParseError};
use nom::branch::alt;
use nom::bytes::complete::{tag, take, take_till, take_until, take_until1, take_while1};
use nom::character::complete::{char, line_ending, multispace0, multispace1, space0, space1};
use nom::combinator::{fail, flat_map, map, not, opt, peek, rest, verify};
use nom::error::Error;
use nom::multi::{separated_list0, separated_list1};
use nom::sequence::{delimited, pair, preceded, separated_pair, terminated, tuple};
use nom::{FindSubstring, IResult, InputTake, Parser};

/// This function parses an inline module, such as `[math latex] x^2`, `[url](https://example.com)`
/// or `[img preview=small]"data.png"`, and returns the parsed module, if successful.
pub fn parse_inline_module(input: &str) -> IResult<&str, Module> {
    map(
        pair(get_module_invocation_parser(true), parse_inline_module_body),
        |((name, args), body)| Module {
            name,
            args,
            body: body.to_string(),
            one_line: true,
        },
    )(input)
}

/// Parses the body of an inline module, which includes possibly parsing the opening delimiter.
/// This first parses an optional opening delimiter, then passes the result to
/// [get_inline_module_parser] which parses the body up until the closing delimiter is found, or
/// a whitespace if no delimiter is found.
fn parse_inline_module_body(input: &str) -> IResult<&str, &str> {
    flat_map(parse_opening_delim(true), get_inline_body_parser)(input)
}

/// Gets a parser for an inline module body, which depends on the delimiter used to open the module,
/// if any. If an opening delimiter is found, it takes all content until a matching closing
/// delimiter is found (not taking any newlines), and if an opening delimiter isn't found, it first
/// discards any whitespace, then takes all consecutive characters until the next whitespace
/// (including counting spaces and newlines).
///
/// # Arguments
///
/// * `delim`: The opening delimiter found, if any
///
/// returns: impl Parser<&str, &str, Error<&str>>+Sized. A parser for the inline body
fn get_inline_body_parser<'a>(
    delim: Option<&'_ str>,
) -> impl Parser<&'a str, &'a str, Error<&'a str>> + '_ {
    move |i: &'a str| {
        if let Some(opening_delim) = delim {
            let closing = closing_delim(opening_delim);
            let res = terminated(
                take_until_no_newlines(closing.as_str()),
                tag(closing.as_str()),
            )(i);
            res
        } else {
            preceded(space0, take_till(|c: char| c.is_ascii_whitespace()))(i)
        }
    }
}

/// This gives a parser which works just like `take_until`, but fails if `take_until` would take a
/// newline. See the documentation for `complete::take_until`. Note that this will use `fail` to
/// generate errors, and thus won't be as useful as the implementation of `take_until`
///
/// # Arguments
///
/// * `tag`: The tag to take
///
/// returns: a parser according to the description above
///
/// # Examples
/// For the tag `eof`:
/// | Input                           | Match         |
/// |---------------------------------|---------------|
/// | `hello, world!eof`              |`hello, world!`|
/// | `hello, \n world!eof"`          |`<Fail>`       |
/// | `hello, world!`                 |`<Fail>`       |
/// | `eof`                           |(empty string) |
///
// this will use take_until to take a substring until a given tag, but won't take any newlines.
// if a newline occurs before the tag, this will fail
// don't mention the body, it is copied from the definition of take_until
fn take_until_no_newlines(tag: &str) -> impl Fn(&str) -> IResult<&str, &str, Error<&str>> + '_ {
    move |i: &str| match i.find_substring(tag) {
        None => fail(i),
        Some(index) => {
            if i.find('\n').map_or(true, |i| i > index) {
                Ok(i.take_split(index))
            } else {
                fail(i)
            }
        }
    }
}

/// Parses optional delimiters for opening and closing modules. If the module is
/// inline, at most one character is allowed, and if it is multiline, any amount
/// of characters can be used.
///
/// # Arguments
///
/// * `inline`: whether the module is inline
///
/// returns: Result<(&str, Option<&str>), Err<Error<I>>>
///
fn parse_opening_delim<'a>(
    inline: bool,
) -> impl Fn(&'a str) -> IResult<&'a str, Option<&'a str>, Error<&'a str>> {
    move |i: &'a str| {
        if inline {
            opt(verify(take(1usize), |s: &str| {
                let c = s.chars().next().unwrap();
                !c.is_alphanumeric() && !c.is_whitespace()
            }))(i)
        } else {
            opt(take_while1(|c: char| {
                !c.is_alphanumeric() && !c.is_whitespace()
            }))(i)
        }
    }
}

/// Gets the appropriate closing delimiter for an opening delimiter for a body of a module
///
/// # Arguments
///
/// * `string`: The opening delimiter
///
/// returns: String
///
/// # Examples
///
/// | Input | Output |
/// |-------|--------|
/// |`---`  | `---`  |
/// |`((`   | `))`   |
/// |`({<*<`| `>*>})`|
fn closing_delim(string: &str) -> String {
    string
        .chars()
        .rev()
        .map(|c| match c {
            '(' => ')',
            '{' => '}',
            '[' => ']',
            '<' => '>',
            '»' => '«',
            '›' => '‹',
            ')' => '(',
            '}' => '{',
            ']' => '[',
            '>' => '<',
            '«' => '»',
            '‹' => '›',
            x => x,
        })
        .collect()
}

/// This function parses a multiline module, such as multiline code blocks, and returns the parse
/// module if successful. This function fails to parse inline modules by design, so that inline
/// modules by themselves in their own paragraphs aren't treated as multiline.
pub fn parse_multiline_module(input: &str) -> IResult<&str, Module> {
    map(
        pair(
            get_module_invocation_parser(false),
            parse_multiline_module_body,
        ),
        |((name, args), body)| Module {
            name,
            args,
            body: body.to_string(),
            one_line: false,
        },
    )(input)
}

/// Parses the body of a multiline module, which includes possibly parsing the opening delimiter.
/// This first parses an optional opening delimiter, then passes the result to
/// [get_multiline_module_parser] which parses the body up until the closing delimiter is found, or
/// two newlines if no delimiter is found.
fn parse_multiline_module_body(input: &str) -> IResult<&str, &str> {
    flat_map(parse_opening_delim(false), get_multiline_body_parser)(input)
}

/// Gets a parser for a multiline module body, which depends on the delimiter used to open the
/// module, if any. If an opening delimiter is found, it takes all content until a matching closing
/// delimiter is found, and if an opening delimiter isn't found, it first discards the first line
/// ending, then takes all consecutive characters until the next double newline.
///
/// # Arguments
///
/// * `delim`: The opening delimiter found, if any
///
/// returns: impl Parser<&str, &str, Error<&str>>+Sized. A parser for the multiline body
fn get_multiline_body_parser<'a>(
    delim: Option<&'_ str>,
) -> impl Parser<&'a str, &'a str, Error<&'a str>> + '_ {
    move |i: &'a str| {
        if let Some(opening_delim) = delim {
            let closing = closing_delim(opening_delim);
            let res = delimited(
                line_ending,
                take_until(closing.as_str()),
                tag(closing.as_str()),
            )(i);
            res
        } else {
            preceded(
                line_ending,
                take_until1("\r\n\r\n").or(take_until1("\n\n").or(rest)),
            )(i)
        }
    }
}

/// Returns a parser for module invocations
///
/// # Arguments
///
/// * `input`: a bool indicating whether the returned parser is used as inline.
///
/// returns: impl Parser<&'a str, (String, ModuleArguments), Error<&'a str>>
///
fn get_module_invocation_parser<'a>(
    inline: bool,
) -> impl Parser<&'a str, (String, MaybeArgs), Error<&'a str>> {
    map(
        delimited(
            char('['),
            pair(
                parse_module_name,
                opt(delimited(
                    get_arg_separator_parser(inline),
                    get_module_args_parser(inline),
                    opt(get_arg_separator_parser(inline)),
                )),
            ),
            char(']'),
        ),
        |(name, args)| (name.to_string(), args.unwrap_or_default()),
    )
}

/// A parser for module names.
///
/// # Arguments
///
/// * `input`: the slice containing the name
///
/// returns: IResult<&str, &str>
///
fn parse_module_name(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c == '-' || c == '_' || c.is_ascii_alphanumeric())(input)
}

/// Returns a parser for parsing module arguments. Works both for named and positional.
/// Both are optional.
///
/// # Arguments
///
/// * `inline`: a bool indicating if the module is inline.
///
/// returns: impl Parser<&str, ModuleArguments, Error<&str>>+Sized
///
fn get_module_args_parser<'a>(inline: bool) -> impl Parser<&'a str, MaybeArgs, Error<&'a str>> {
    map(
        opt(alt((
            map(
                tuple((
                    separated_list0(
                        get_arg_separator_parser(inline),
                        get_unnamed_arg_parser(inline),
                    ),
                    opt(get_arg_separator_parser(inline)),
                    separated_list1(
                        get_arg_separator_parser(inline),
                        get_named_arg_parser(inline),
                    ),
                    get_arg_separator_parser(inline),
                    separated_list1(
                        get_arg_separator_parser(inline),
                        get_unnamed_arg_parser(inline),
                    ),
                )),
                |_tuple| MaybeArgs::Error(ParseError::ArgumentOrderError),
            ),
            map(
                separated_pair(
                    separated_list1(
                        get_arg_separator_parser(inline),
                        get_unnamed_arg_parser(inline),
                    ),
                    get_arg_separator_parser(inline),
                    separated_list1(
                        get_arg_separator_parser(inline),
                        get_named_arg_parser(inline),
                    ),
                ),
                |(unnamed, named)| {
                    MaybeArgs::ModuleArguments(ModuleArguments {
                        positioned: Some(unnamed),
                        named: Some(named.into_iter().collect()),
                    })
                },
            ),
            map(
                separated_list1(
                    get_arg_separator_parser(inline),
                    get_unnamed_arg_parser(inline),
                ),
                |unnamed| {
                    MaybeArgs::ModuleArguments(ModuleArguments {
                        positioned: Some(unnamed),
                        named: None,
                    })
                },
            ),
            map(
                separated_list1(
                    get_arg_separator_parser(inline),
                    get_named_arg_parser(inline),
                ),
                |named| {
                    MaybeArgs::ModuleArguments(ModuleArguments {
                        positioned: None,
                        named: Some(named.into_iter().collect()),
                    })
                },
            ),
        ))),
        |x| x.unwrap_or_default(),
    )
}

/// Returns a parser parsing the separator of arguments. The separators are whitespace and optional comma.
///
/// # Arguments
///
/// * `inline`: a bool indicating if the parser is for inline
///
/// returns: impl Parser<&str, (), Error<&str>>+Sized
///

fn get_arg_separator_parser<'a>(inline: bool) -> impl Parser<&'a str, (), Error<&'a str>> {
    let space = if inline { space1 } else { multispace1 };
    map(space, |_| ())
}

/// Parses the optional unnamed args on all unnamed arguments removing arg separators
///
/// # Arguments
///
/// * `inline`: All unnamed args
///
/// returns: impl Parser<&str, String, Error<&str>>+Sized
///
fn get_unnamed_arg_parser<'a>(inline: bool) -> impl Parser<&'a str, String, Error<&'a str>> {
    map(
        terminated(arg_value_parser, peek(not(get_kv_separator_parser(inline)))),
        |s| s.to_string(),
    )
}

/// Gets a parser which parses a named argument, eg `lang = python`, and returns a key-value pair
/// of owned Strings.
///
/// See `arg_name_parser`, `arg_value_parser` and `get_kv_separator_parser` for more info about
/// rules for argument names, values and separators
///
/// # Arguments
///
/// * `inline`: if the inline or multiline ruleset should be followed
///
/// # Examples
///
/// | Input                           | Match             |
/// |---------------------------------|-------------------|
/// | `apple=pie`                     | `(apple, pie)`    |
/// | `delim = "yes box"`             | `(delim, yes box)`|
/// | `"fake" = news`                 | `<Fail>`          |
/// | `<space>`                       | `<Fail>`          |
/// returns: a parser parsing one named argument
///
fn get_named_arg_parser<'a>(
    inline: bool,
) -> impl Parser<&'a str, (String, String), Error<&'a str>> {
    map(
        separated_pair(
            arg_key_parser,
            get_kv_separator_parser(inline),
            arg_value_parser,
        ),
        |(a, b)| (a.to_string(), b.to_string()),
    )
}

/// Parses an argument key and returns it. It may contain any alphanumeric characters and
/// underscores. It consumes the captured argument name, and nothing more
///
/// # Arguments
///
/// * `input`: The string to parse
///
/// # Examples
///
/// | Input                           | Match     |
/// |---------------------------------|-----------|
/// | `apple=pie`                     | `apple`   |
/// | `delim = "yes box"`             | `delim`   |
/// | `"fake" = news`                 | `<Fail>`  |
/// | `<space>`                       | `<Fail>`  |
///
/// returns: The parsing result capturing the argument name
fn arg_key_parser(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)
}

/// Parses the argument to a function removing optional quotation marks and returning the value.
///
/// # Arguments
///
/// * `input`: The string to parse
///
/// # Examples
///
/// | Input                           | Match          |
/// |---------------------------------|----------------|
/// | `python 3`                      | `python`       |
/// | `"Alice Parker" "Matt Steward"` | `Alice Parker` |
/// | `a_b_c_d e_f_g_h`               | `a_b_c_d`      |
/// | `!"#!€/"(`                      | `<Error>`      |
///
/// returns: a parser consuming and returning the match
///
fn arg_value_parser(input: &str) -> IResult<&str, &str> {
    alt((
        delimited(char('"'), take_until1(r#"""#), char('"')),
        take_while1(|c: char| c.is_ascii_alphanumeric() || c == '_'),
    ))(input)
}

/// Gets a parser which consumes the key-value separator, `=` in `lang=python`, without returning
/// anything, and failing if the consumption failed
///
/// For inline, this is defined as `[ \t]*=[ \t]*`, and for multiline, this is defined as
/// `[ \t\r\n]*=[ \t\r\n]*`
///
/// # Arguments
///
/// * `inline`: a bool indicating if the parser is using the ruleset for inline modules
///
/// # Examples:
///
/// | Input (il=inline, ml=multiline) | Match          |
/// |---------------------------------|----------------|
/// | `banana`                        | `<Fail>`       |
/// | `<space>=<space>` (il/ml)       | `<Success>`    |
/// | `\n\n<space>=<space>\t\n` (ml)  | `<Success>`    |
/// | `<space>`                       | `<Fail>`       |
///
/// returns: a parser consuming but not returning the match
///
fn get_kv_separator_parser<'a>(inline: bool) -> impl Parser<&'a str, (), Error<&'a str>> {
    let space = if inline { space0 } else { multispace0 };
    map(delimited(space, char('='), space), |_| ())
}
