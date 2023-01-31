// Just some placeholder code. We will decide how to structure
// our parser and what internal representation to use later :)

use std::collections::HashMap;
use std::mem;

use lazy_static::lazy_static;
use regex::{Match, Regex};

use crate::ModuleArguments::{Named, Positioned};

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
        args: Option<ModuleArguments>,
        body: String,
        one_line: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModuleArguments {
    Positioned(Vec<String>),
    Named(HashMap<String, String>),
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

        Element::ModuleInvocation {
            name,
            args,
            body,
            one_line,
        } => {
            //format like:
            //name(args) {
            //  body
            //}
            let args: String = match args {
                None => "".to_string(),
                Some(args) => match args {
                    Positioned(args) => args.join(", "),
                    Named(args) => args.iter().map(|(k, v)| format!("{k}={v}")).fold(
                        "".to_string(),
                        |acc: String, arg: String| {
                            if arg.is_empty() {
                                arg
                            } else {
                                format!("{acc}, {arg}")
                            }
                        },
                    ),
                },
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

fn parse_args(string: &str) -> Result<Option<ModuleArguments>, ()> {
    lazy_static! {
        static ref UNNAMED_ARGS: Regex = Regex::new(
            r#"(?xs)  # enable whitespace insensitivity and singleline (dotall)
            \s*       # ignore leading whitespace
            ([\w\d]+| # one single alphanumeric word, or
            "         # open quote,
            (?:"|     # and closing quote or
            .*?       # as few characters as possible,
            [^\\]     # followed by an even number of
            (?:\\\\)* # backslashes,
            "))       # and an endquote,
            \s*,?     # possibly whitespace and a comma
            "#
        )
        .unwrap();
        static ref NAMED_ARGS: Regex = Regex::new(
            r#"(?xs)  # enable whitespace insensitivity and singleline (dotall)
            \s*       # ignore leading whitespace
            ([\w\d]+) # one single alphanumeric word,
            \s*=\s*   # possibly whitespace, equals sign, and possibly whitespace, followed by
            ([\w\d]+| # one single alphanumeric word, or
            "         # open quote,
            (?:"|     # and closing quote or
            .*?       # as few characters as possible,
            [^\\]     # followed by an even number of
            (?:\\\\)* # backslashes,
            "))       # and an endquote,
            \s*,?     # possibly whitespace and a comma
            "#
        )
        .unwrap();
    }

    return Ok(Some(Positioned(vec!["abc".to_string()])));

    if string.trim().is_empty() {
        return Ok(None);
    }

    let mut success = false;
    let mut unnamed_args = vec![];
    let mut prev_find = 0;

    for cap in UNNAMED_ARGS.captures_iter(string) {
        let full_range = cap.get(0).unwrap().range();
        if prev_find <= full_range.start {
            prev_find = full_range.end;
        } else {
            success = false;
            break;
        }

        let range = cap.get(1).unwrap().range();

        if string[range.end..].trim_end().is_empty() {
            success = true;
        }
        unnamed_args.push(string[range].to_string());
    }

    if success {
        return Ok(Some(Positioned(unnamed_args)));
    }

    prev_find = 0;
    let mut named_args: HashMap<String, String> = HashMap::new();

    for cap in NAMED_ARGS.captures_iter(string) {
        let full_range = cap.get(0).unwrap().range();
        let name_range = cap.get(1).unwrap().range();
        let val_range = cap.get(2).unwrap().range();

        if prev_find <= full_range.start {
            prev_find = full_range.end;
        } else {
            return Err(());
        }

        if string[val_range.end..].trim_end().is_empty() {
            success = true;
        }

        named_args.insert(
            string[name_range].to_string(),
            string[val_range].to_string(),
        );
    }

    if success {
        Ok(Some(Named(named_args)))
    } else {
        Err(())
    }
}

pub fn parse(source: &str) -> Element {
    let mut doc: Element = Element::Node {
        name: "Document".into(),
        attributes: HashMap::new(),
        children: vec![],
    };

    lazy_static! {
        //^.*(?:^|[^\\])\\(?:\\\\)*(\r?\n)
        static ref ESCAPED_LINE_REMOVAL: Regex = Regex::new(
            r"(?x)      # enable whitespace insensitity
            ^.*         # capture any amount of characters from the start of the line
            (?:^|[^\\]) # and a non-backslash (or the start of the line)
            (?:\\)      # and one backslash
            (?:\\\\)*   # followed by an even amount of backslashes
            (\r?\n)     # followed by a CRLF/LF (which is caputred)
            "
        ).unwrap();

        //^(?:[^\r\n]*?[^\\](?:\\\\)*)??\[\s*(\S+)(?:\s*(.*?[^\\](?:\\\\)*))??\]([^\w\d\s\r\n\])}])*
        static ref MODULE_BLOCK: Regex = Regex::new(
            r"(?xs)          # enable whitespace insensitivity and singleline (dotall)
            ^                # start at the start of the string
            (?:[^\r\n]*?     # take as few characters as possible (in the same line)
            [^\\\n](?:\\\\)* # ...which doesn't end in an odd number of backslashes
            )??              # ...possibly (and possibly not taking any characters), but lazily
            \[               # then, find an open bracket
            \s*?             # and skip any whitespace (including newlines)
            ([\w\d-]+)       # then, take alphanumeric name (allowing dashes, captured in group 1)
            (?:\s*?          # then, ignore all whitespace,
            (.*?             # capture everything, as little as possible
            [^\\](?:\\\\)*   # ending in an even number of backslashes,
            ))??             # if possible, but preferably not
            \]               # then, find the closing bracket
            ([^\w\d          # immediately after, capture all non-alphanumeric,
            \s\r\n           # non-whitespace, non-newlines
            \])}]*           # non-opening-brackets, as many as possible
            )                # and capture in group 2 (this will be our potential open/close delim)
            "
        ).unwrap();

        static ref MODULE_BLOCK_WITHOUT_DELIMETER: Regex = Regex::new(
            r"(?xs)
            ^            # start at the start of the string
            \x20*        # optionally multiple spaces
            (?:
            (            # capture the body
            \S+?         # either multiple non-whitespace characters...
            )(?:[\x20\r\n]|$) # followed by a space or newline or end of string, or
            |\r?\n       # or a newline
            (.*?)        # and the body is all characters
            (?:
            (?:\r?\n){2} # until two newlines
            |$)          # or the end of the string
            )
            "
        ).unwrap();

        static ref ESCAPED_NEW_LINE_BLOCK_SEQUENCE: Regex = Regex::new(
            r"(?x)
            (\r?\n)
            '('*)
            (\r?\n)
            "
        ).unwrap();
    }

    let default_paragraph: Element = Element::Node {
        name: "Paragraph".into(),
        attributes: HashMap::new(),
        children: vec![],
    };

    let mut current_paragraph: Element = default_paragraph.clone();

    let mut input_str = source.to_string();

    fn parse_normal(string: &str) -> Element {
        Element::Data(string.to_string())
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

    let mut push_to_paragraph = |elem: Element| {
        if let Element::Node {
            name: _name,
            attributes: _attributes,
            children,
        } = &mut current_paragraph
        {
            children.push(elem);
        }
    };

    while !input_str.is_empty() {
        // Remove all escaped newlines
        while let Some(captures) = ESCAPED_LINE_REMOVAL.captures(&input_str) {
            let group = captures.get(1).unwrap(); // this is the newline, CRLF/LF
            // remove the backslash
            input_str.replace_range(group.start() - 1..group.end(), "");
        }

        // Check if there is a module invocation
        if let Some(mod_inv) = MODULE_BLOCK.captures(&input_str.clone()) {
            // Get the content before the module invocation
            // Module invocation starts with backslash, so -1 to get rid of it
            let before = input_str[..mod_inv.get(1).unwrap().start() - 1].to_string();
            // If some, push it to paragraph
            if !before.is_empty() {
                push_to_paragraph(parse_normal(&before));
            }

            //find out the name, if it ends with comma, it is delimiting the args so remove it
            let mut name = input_str[mod_inv.get(1).unwrap().range()].to_string();
            if name.ends_with(',') {
                name.remove(name.len() - 1);
            }

            let args = if let Some(arg_match) = mod_inv.get(2) {
                //parse_args(&input_str[arg_match.range()]).unwrap();
                Some(Positioned(vec![input_str[arg_match.range()].to_string()]))
            } else {
                None
            };

            let mut elem: Option<Element> = None;

            let delimiter = input_str[mod_inv.get(3).unwrap().range()].to_string();

            input_str.replace_range(mod_inv.get(0).unwrap().range(), "");

            if delimiter.is_empty() {
                if let Some(captures) = MODULE_BLOCK_WITHOUT_DELIMETER.captures(&input_str) {
                    let (one_line, body_range) = if let Some(c) = captures.get(1) {
                        (true, c.range())
                    } else if let Some(c) = captures.get(2) {
                        (false, c.range())
                    } else {
                        panic!("No module body found even though match was successful");
                    };

                    let body = ESCAPED_NEW_LINE_BLOCK_SEQUENCE
                        .replace_all(&input_str[body_range.clone()], "$1$2$3")
                        .to_string();

                    elem = Some(Element::ModuleInvocation {
                        name,
                        args,
                        body,
                        one_line,
                    });
                    input_str.replace_range(captures.get(0).unwrap().range(), "");
                } else {
                    println!("Could not find body of block with name {name}");
                    elem = Some(Element::ModuleInvocation {
                        name,
                        args,
                        body: "".to_string(),
                        one_line: true,
                    })
                }
            } else {
                let end_delim = closing_delim(&delimiter);
                let body_range =
                    ..input_str.find(&end_delim).unwrap_or(input_str.len());
                let body = input_str[body_range].to_string();
                let one_line = !body.contains('\n');
                
                elem = Some(Element::ModuleInvocation {
                    name,
                    args,
                    body,
                    one_line
                });
                
                let end_limit = body_range.end + end_delim.len();
                
                if end_limit >= input_str.len() {
                    input_str = "".to_string();
                } else {
                    input_str.replace_range(
                        .. end_limit, "");
                }
            }
            push_to_paragraph(elem.unwrap());
        } else {
            // there isn't a module invocation -> parse content as normal
            // find first newline
            let nl = input_str.find('\n').unwrap_or(input_str.len());
            push_to_paragraph(Element::Data(input_str[..nl].to_string()));
            if nl == input_str.len() {
                input_str = "".to_string();
            } else {
                input_str.replace_range(..=nl, "");
            }
        }
    }

    if let Element::Node {
        name: _,
        attributes: _,
        children,
    } = &mut doc
    {
        children.push(current_paragraph)
    }

    doc
}
