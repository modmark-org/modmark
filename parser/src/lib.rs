extern crate core;

use std::collections::HashMap;
use std::mem;

use nom::bytes::complete::{is_a, take_till, take_until, take_until1, take_while1};
use nom::character::complete::{
    alphanumeric1, char, line_ending, multispace0, multispace1, none_of, space0, space1,
};
use nom::error::Error;
use nom::multi::{fold_many1, many1, separated_list0, separated_list1};
use nom::sequence::{delimited, pair, preceded, separated_pair, terminated};
use nom::{
    branch::*, bytes::complete::tag, combinator::*, FindSubstring, IResult, InputTake, Parser,
};

use Element::Node;

use crate::Element::{Data, ModuleInvocation};

mod or;

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Data(String),
    Node {
        name: String,
        environment: HashMap<String, String>,
        children: Vec<Element>,
    },
    ModuleInvocation {
        name: String,
        args: ModuleArguments,
        body: String,
        one_line: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ModuleArguments {
    positioned: Option<Vec<String>>,
    named: Option<HashMap<String, String>>,
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

/// Parses the argument value of a function removing optional quotation marks and returning the value.
///
/// # Arguments
///
/// * `input`: An argument
///
/// returns: Result<(&str, &str), Err<Error<I>>>
///
fn arg_value_parser(input: &str) -> IResult<&str, &str> {
    alt((
        delimited(char('"'), take_until1(r#"""#), char('"')),
        take_while1(|c: char| c.is_ascii_alphanumeric() || c == '_'),
    ))(input)
}

/// Returns a parser parsing separators for key-value pairs
///
/// # Arguments
///
/// * `inline`: a bool indicating if the parser is for inline
///
/// returns: impl Parser<&'a str, (), Error<&'a str, (), Error<&'a str>>
///
fn get_kv_separator_parser<'a>(inline: bool) -> impl Parser<&'a str, (), Error<&'a str>> {
    let space = if inline { space0 } else { multispace0 };
    map(delimited(space, char('='), space), |_| ())
}

/// A parser for named arguments
///
/// # Arguments
///
/// * `input`: the slice containing the arguments
///
/// returns: IResult<&str, (String, String)>
///
// TODO: make two versions for inline/not inline
fn named_arg(input: &str) -> IResult<&str, (String, String)> {
    map(
        separated_pair(
            arg_name_parser,
            get_kv_separator_parser(true),
            arg_value_parser,
        ),
        |(a, b)| (a.to_string(), b.to_string()),
    )(input)
}

fn arg_name_parser(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)
}

/// Parses the optional unnamed args on all unnamed arguments removing arg separators
///
/// # Arguments
///
/// * `inline`: All unnamed args
///
/// returns: impl Parser<&str, String, Error<&str>>+Sized
///
fn unnamed_arg<'a>(inline: bool) -> impl Parser<&'a str, String, Error<&'a str>> {
    map(
        terminated(arg_value_parser, peek(not(get_kv_separator_parser(inline)))),
        |s| s.to_string(),
    )
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
fn get_module_args_parser<'a>(
    inline: bool,
) -> impl Parser<&'a str, ModuleArguments, Error<&'a str>> {
    map(
        opt(alt((
            map(
                separated_pair(
                    separated_list1(get_arg_separator_parser(inline), unnamed_arg(inline)),
                    get_arg_separator_parser(inline),
                    separated_list1(get_arg_separator_parser(inline), named_arg),
                ),
                |(unnamed, named)| ModuleArguments {
                    positioned: Some(unnamed),
                    named: Some(named.into_iter().collect()),
                },
            ),
            map(
                separated_list1(get_arg_separator_parser(inline), unnamed_arg(inline)),
                |unnamed| ModuleArguments {
                    positioned: Some(unnamed),
                    named: None,
                },
            ),
            map(
                separated_list1(get_arg_separator_parser(inline), named_arg),
                |named| ModuleArguments {
                    positioned: None,
                    named: Some(named.into_iter().collect()),
                },
            ),
        ))),
        |x| x.unwrap_or_default(),
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
) -> impl Parser<&'a str, (String, ModuleArguments), Error<&'a str>> {
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

/// Parses optional delimitors for opening and closing modules.
///
/// # Arguments
///
/// * `input`:
///
/// returns: Result<(&str, Option<&str>), Err<Error<I>>>
///
fn parse_opening_delim(input: &str) -> IResult<&str, Option<&str>> {
    opt(take_while1(|c: char| {
        !c.is_alphanumeric() && !c.is_whitespace()
    }))(input)
}

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

/*fn until_tag_without_line_endings<'a>(
    tag: &'a str,
) -> impl Parser<&'a str, &'a str, Error<&'a str>> + '_ {
    move |i: &'a str| {
        let t = <&str>::clone(&tag);
        let until_res = take_until(t)(i);
        if let Ok((_, taken)) = until_res {
            let nl_pos = i.find('\n');
            if nl_pos.map_or(true, |i| i > taken.len()) {
                until_res
            } else {
                fail(i)
            }
        } else {
            until_res
        }
    }
}*/

/// This is a parser which works just like `take_until`, but fails if `take_until` would take a
/// newline. See the documentation for `complete::take_until`. Note that this will use `fail` to
/// generate errors, and thus won't be as useful as the implementation of `take_until`
///
/// # Arguments
///
/// * `tag`: The tag to take
///
/// returns: impl Fn(&str) -> Result<(&str, &str), Err<Error<&str>>>+Sized
///
/// # Examples
///
/// ```rust,ignore
/// fn until_eof(s: &str) -> IResult<&str, &str> {
///   take_until_no_newlines("eof")(s)
/// }
///
/// // until_eof("hello, worldeof") -> Ok(("eof", "hello, world"))
/// // until_eof("hello,\n worldeof") -> Err(Err::Error(ErrorKind::Fail))
/// // until_eof("hello, world") -> Err(Err::Error(ErrorKind::Fail))
/// ```
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

fn parse_inline_module_body(input: &str) -> IResult<&str, &str> {
    flat_map(parse_opening_delim, get_inline_body_parser)(input)
}

fn parse_inline_module(input: &str) -> IResult<&str, Element> {
    map(
        pair(get_module_invocation_parser(true), parse_inline_module_body),
        |((name, args), body)| ModuleInvocation {
            name,
            args,
            body: body.to_string(),
            one_line: true,
        },
    )(input)
}

fn parse_multiline_module_body(input: &str) -> IResult<&str, &str> {
    flat_map(parse_opening_delim, get_multiline_body_parser)(input)
}

fn parse_multiline_module(input: &str) -> IResult<&str, Element> {
    map(
        pair(
            get_module_invocation_parser(false),
            parse_multiline_module_body,
        ),
        |((name, args), body)| ModuleInvocation {
            name,
            args,
            body: body.to_string(),
            one_line: false,
        },
    )(input)
}

fn escape(char: char) -> char {
    char
}

fn parse_paragraph_elements(input: &str) -> IResult<&str, Vec<Element>> {
    map(
        fold_many1(
            or::or5(
                parse_inline_module,
                preceded(char('\\'), line_ending),
                preceded(char('\\'), none_of("\r\n")),
                none_of("\r\n"),
                // note: do NOT use not_line_ending, it matches successfully on empty string
                // so that would break this
                terminated(line_ending, peek(none_of("\r\n"))),
            ),
            || (Vec::new(), String::new()),
            |(acc_vec, acc_str), (opt_inline, _opt_esc_line_ending, opt_esc_char, opt_char, opt_line_ending)| {
                let mut elems = acc_vec;
                let mut string = acc_str;

                if let Some(module) = opt_inline {
                    if !string.is_empty() {
                        elems.push(Data(mem::take(&mut string)))
                    }
                    elems.push(module);
                } else if let Some(esc_char) = opt_esc_char {
                    string.push(escape(esc_char));
                } else if let Some(n_char) = opt_char {
                    string.push(n_char);
                } else if let Some(line_ending) = opt_line_ending {
                    string.push_str(line_ending);
                }
                (elems, string)
            },
        ),
        |(a, b)| {
            let mut elems = a;
            if !b.is_empty() {
                elems.push(Data(b))
            }
            elems
        },
    )(input)
}

/// Parses a paragraph which consists of multiple paragraph elements, and puts all those into a
/// `Paragraph` node.
///
/// # Arguments
///
/// * `input`:
///
/// returns: Result<(&str, Element), Err<Error<I>>>
fn parse_paragraph(input: &str) -> IResult<&str, Element> {
    map(parse_paragraph_elements, |elems| Node {
        name: "Paragraph".to_string(),
        environment: Default::default(),
        children: elems,
    })(input)
}

/// Parses multiple paragraphs or multiline modules, separated by two or more line endings.
/// The result will be a vector of the elements parsed, where each element is either a
/// multiline module invocation or a `Paragraph` node.
///
/// # Arguments
///
/// * `input`: The text to parse
///
/// returns: Result<(&str, Vec<Element, Global>), Err<Error<I>>>
fn parse_multiple_paragraphs(input: &str) -> IResult<&str, Vec<Element>> {
    separated_list0(
        preceded(line_ending, many1(line_ending)),
        parse_multiline_module.or(parse_paragraph),
    )(input)
}

/// Parses a document, which consists of multiple paragraphs and block modules, and returns a
/// `Node` with the name `Document` containing all paragraphs
///
/// # Arguments
///
/// * `input`: The input to parse
///
/// returns: Result<(&str, Element), Err<Error<I>>>
fn parse_document(input: &str) -> IResult<&str, Element> {
    map(parse_multiple_paragraphs, |paras| Node {
        name: "Document".to_string(),
        environment: Default::default(),
        children: paras,
    })(input)
}

fn do_parse(input: &str) -> Element {
    let res = parse_document(input);
    match &res {
        Ok(_) => {}
        Err(x) => {
            dbg!(x);
        }
    }

    println!("{:?}", res.as_ref().unwrap());
    res.unwrap().1
}

fn closing_delim(string: &str) -> String {
    string
        .chars()
        .rev()
        .map(|c| match c {
            '(' => ')',
            '{' => '}',
            '[' => ']',
            '<' => '>',
            x => x,
        })
        .collect()
}

pub fn parse(source: &str) -> Element {
    do_parse(source)
}

impl Element {
    pub fn tree_string(&self, include_environment: bool) -> String {
        pretty_rows(self, include_environment).join("\n")
    }
}

/// Converts an AST into a vector of strings structured for printing.
///
/// # Arguments
///
/// * `element`: A pointer to the root of the tree
/// * `include_environment`:
///
/// returns: Result<(&str, Vec<Element, Global>), Err<Error<I>>>
fn pretty_rows(element: &Element, include_environment: bool) -> Vec<String> {
    let indent = "  ";
    let mut strs = vec![];

    match element {
        Data(str) => str.lines().enumerate().for_each(|(idx, line)| {
            strs.push(format!("{} {line}", if idx == 0 { '>' } else { '|' }))
        }),
        Node {
            name,
            environment,
            children,
        } => {
            strs.push(format!("{name} {{"));
            if environment.is_empty() {
                strs.push(format!("{indent}env: {{ <empty> }}"));
            } else if include_environment {
                strs.push(format!("{indent}env: {{"));
                environment
                    .iter()
                    .for_each(|(k, v)| strs.push(format!(r#"{indent}{indent}"{k}": "{v}""#)));

                strs.push(format!("{indent}}}"));
            } else {
                strs.push(format!(
                    "{indent}env: {{ < {len} entries > }}",
                    len = &environment.len().to_string()
                ))
            }

            if children.is_empty() {
                strs.push(format!("{indent}children: [ none ]"));
            } else {
                strs.push(format!("{indent}children: ["));

                children.iter().for_each(|c| {
                    pretty_rows(c, include_environment)
                        .iter()
                        .for_each(|s| strs.push(format!("{indent}{indent}{s}")))
                });

                strs.push(format!("{indent}]"));
            }
            strs.push("}".to_string());
        }

        ModuleInvocation {
            name,
            args,
            body,
            one_line,
        } => {
            let args = {
                let p1 = &args.positioned;
                let p2 = args.named.as_ref().map(|args| {
                    args.iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<String>>()
                });

                let mut args_vec = p1.clone().unwrap_or_default();
                args_vec.extend_from_slice(&p2.unwrap_or_default());
                args_vec.join(", ")
            };
            if *one_line {
                strs.push(format!("{name}({args}){{{body}}}"));
            } else {
                strs.push(format!("{name}({args}){{"));
                body.lines().enumerate().for_each(|(idx, line)| {
                    strs.push(format!(
                        "{indent}{} {line}",
                        if idx == 0 { '>' } else { '|' }
                    ))
                });
                strs.push("} [multiline invocation]".to_string());
            }
        }
    }
    strs
}
