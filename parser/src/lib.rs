// Just some placeholder code. We will decide how to structure
// our parser and what internal representation to use later :)

extern crate core;

use std::collections::HashMap;

use std::mem;

use nom::bytes::complete::{take_till, take_until, take_until1, take_while1};
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

fn arg_value_parser(input: &str) -> IResult<&str, &str> {
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
        map(arg_value_parser, |x| x.to_string()),
    )(input)
}

fn unnamed_arg<'a>(inline: bool) -> impl Parser<&'a str, String, Error<&'a str>> {
    map(
        terminated(arg_value_parser, peek(not(get_kv_separator_parser(inline)))),
        |s| s.to_string(),
    )
}

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

fn parse_module_name(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c == '-' || c == '_' || c.is_ascii_alphanumeric())(input)
}

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
                terminated(line_ending, peek(none_of("\r\n")))
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

fn parse_paragraph(input: &str) -> IResult<&str, Element> {
    map(parse_paragraph_elements, |elems| Node {
        name: "Paragraph".to_string(),
        attributes: Default::default(),
        children: elems,
    })(input)
}

fn parse_multiple_paragraphs(input: &str) -> IResult<&str, Vec<Element>> {
    separated_list0(
        preceded(line_ending, many1(line_ending)),
        parse_multiline_module.or(parse_paragraph),
    )(input)
    //many0(parse_paragraph)(input)
}

fn parse_document(input: &str) -> IResult<&str, Element> {
    map(parse_multiple_paragraphs, |paras| Node {
        name: "Document".to_string(),
        attributes: Default::default(),
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
    /*let actual_res = res.map(|(_, elems)| Element::Node {
        name: "Document".to_string(),
        attributes: HashMap::new(),
        children: elems,
    });
    return actual_res.unwrap();*/
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
    pub fn tree_string(&self, include_attributes: bool) -> String {
        pretty_rows(self, include_attributes).join("\n")
    }
}

fn pretty_rows(element: &Element, include_attributes: bool) -> Vec<String> {
    let indent = "  ";
    let mut strs = vec![];

    match element {
        Data(str) => str.lines().enumerate().for_each(|(idx, line)| {
            strs.push(format!("{} {line}", if idx == 0 { '>' } else { '|' }))
        }),
        Node {
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
