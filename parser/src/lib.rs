// Just some placeholder code. We will decide how to structure
// our parser and what internal representation to use later :)

extern crate core;

use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::mem;

use lazy_static::lazy_static;
use nom::bytes::complete::{
    is_a, is_not, take_till, take_till1, take_until, take_until1, take_while, take_while1,
};
use nom::character::complete::{
    alphanumeric1, anychar, char, multispace0, multispace1, newline, none_of, not_line_ending,
    space0, space1,
};
use nom::character::{is_alphanumeric, is_space};
use nom::error::Error;
use nom::multi::{fold_many0, fold_many1, many0, many1, separated_list0, separated_list1};
use nom::sequence::{delimited, pair, preceded, separated_pair, terminated};
use nom::{
    branch::*,
    bytes::complete::{tag, take_while_m_n},
    combinator::*,
    sequence::tuple,
    Finish, IResult, Parser,
};

use crate::Element::{Data, ModuleInvocation};

mod or;

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Data(String),
    Node {
        name: String,
        attributes: HashMap<String, String>,
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

fn get_arg_separator_parser<'a>(inline: bool) -> impl Parser<&'a str, (), Error<&'a str>> {
    let space = if inline { space1 } else { multispace1 };
    map(pair(opt(char(',')), space), |_| ())
}

/*fn parse_inline_arg_separator(input: &str) -> IResult<&str, ()> {
    map(pair(opt(char(',')), space1), |_| ())(input)
}

fn parse_multiline_arg_separator(input: &str) -> IResult<&str, ()> {
    map(pair(opt(char(',')), multispace1), |_| ())(input)
}*/

fn parse_arg_value(input: &str) -> IResult<&str, &str> {
    alt((
        delimited(char('"'), take_until1(r#"""#), char('"')),
        take_while1(|c: char| c.is_ascii_alphanumeric()),
    ))(input)
}

fn get_kv_separator_parser<'a>(inline: bool) -> impl Parser<&'a str, (), Error<&'a str>> {
    let space = if inline { space0 } else { multispace0 };
    map(delimited(space, char('='), space), |_| ())
}

fn named_arg(input: &str) -> IResult<&str, (String, String)> {
    separated_pair(
        map(many1(alt((alphanumeric1, tag("_")))), |cs| cs.join("")),
        get_kv_separator_parser(true),
        map(parse_arg_value, |x| x.to_string()),
    )(input)
}

fn unnamed_arg<'a>(inline: bool) -> impl Parser<&'a str, String, Error<&'a str>> {
    map(
        terminated(parse_arg_value, peek(not(get_kv_separator_parser(inline)))),
        |s| s.to_string(),
    )
}

fn get_module_args_parser<'a>(inline: bool) -> impl Parser<&'a str, ModuleArguments, Error<&'a str>> {
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

fn parse_module_name(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c == '-' || c == '_' || c.is_ascii_alphanumeric())(input)
}

fn parse_inline_module_invocation(input: &str) -> IResult<&str, (String, ModuleArguments)> {
    map(
        delimited(
            char('['),
            pair(
                parse_module_name,
                opt(preceded(
                    get_arg_separator_parser(true),
                    get_module_args_parser(true),
                )),
            ),
            char(']'),
        ),
        |(name, args)| (name.to_string(), args.unwrap_or_default()),
    )(input)
}

fn parse_opening_delim(input: &str) -> IResult<&str, Option<&str>> {
    opt(take_while1(|c: char| {
        !c.is_alphanumeric() && !c.is_whitespace()
    }))(input)
}

fn take_body_helper<'a>(
    delim: Option<&'_ str>,
) -> impl Fn(&'a str) -> IResult<&'a str, &'a str> + '_ {
    move |i: &'a str| {
        if let Some(opening_delim) = delim {
            let closing = closing_delim(opening_delim);
            let res = terminated(take_until(closing.as_str()), tag(closing.as_str()))(i);
            res
        } else {
            preceded(space0, take_till(|c: char| c.is_ascii_whitespace()))(i)
        }
    }
}

fn parse_inline_module_body(input: &str) -> IResult<&str, &str> {
    flat_map(parse_opening_delim, take_body_helper)(input)
}

fn parse_inline_module(input: &str) -> IResult<&str, Element> {
    map(
        pair(parse_inline_module_invocation, parse_inline_module_body),
        |((name, args), body)| ModuleInvocation {
            name,
            args,
            body: body.to_string(),
            one_line: true,
        },
    )(input)
}

fn escape(char: char) -> char {
    char
}

fn parse_line(input: &str) -> IResult<&str, Vec<Element>> {
    map(
        fold_many1(
            or::or4(
                parse_inline_module,
                preceded(char('\\'), newline),
                preceded(char('\\'), none_of("\r\n")),
                none_of("\r\n"),
            ),
            || (Vec::new(), String::new()),
            |(acc_vec, acc_str), (opt_inline, opt_esc_newline, opt_esc_char, opt_char)| {
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

fn do_parse(input: &str) -> Element {
    let res = parse_line(input);
    match &res {
        Ok(_) => {}
        Err(x) => {
            dbg!(x);
        }
    }

    println!("{:?}", res.as_ref().unwrap());
    let actual_res = res.map(|(_, elems)| Element::Node {
        name: "Document".to_string(),
        attributes: HashMap::new(),
        children: elems,
    });
    return actual_res.unwrap();
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
    pub fn tree_string(&self, include_attributes: bool) -> String {
        pretty_rows(self, include_attributes).join("\n")
    }
}

fn pretty_rows(element: &Element, include_attributes: bool) -> Vec<String> {
    let indent = "  ";
    let mut strs = vec![];

    match element {
        Element::Data(str) => strs.push(format!(r#""{str}""#)),
        Element::Node {
            name,
            attributes,
            children,
        } => {
            strs.push(format!("{name} {{"));
            if attributes.is_empty() {
                strs.push(format!("{indent}attributes: {{ <empty> }}"));
            } else if include_attributes {
                strs.push(format!("{indent}attributes: {{"));

                attributes
                    .iter()
                    .for_each(|(k, v)| strs.push(format!(r#"{indent}{indent}"{k}": "{v}""#)));

                strs.push(format!("{indent}}}"));
            } else {
                strs.push(format!(
                    "{indent}attributes: {{ < {len} attributes > }}",
                    len = &attributes.len().to_string()
                ))
            }

            if children.is_empty() {
                strs.push(format!("{indent}children: [ none ]"));
            } else {
                strs.push(format!("{indent}children: ["));

                children.iter().for_each(|c| {
                    pretty_rows(c, include_attributes)
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
            //format like:
            //name(args) {
            //  body
            //}
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
            strs.push(format!("{name}({args}){{"));
            body.lines()
                .for_each(|line| strs.push(format!("{indent}{line}")));
            strs.push(if *one_line {
                "} [one-line invocation]".to_string()
            } else {
                "} [multiline invocation]".to_string()
            });
        }
    }
    strs
}
