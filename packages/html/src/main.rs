use std::{
    collections::HashMap,
    env,
    fmt::Write,
    io::{self, Read},
};

use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, to_value, Value};

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum JsonEntry {
    ParentNode {
        name: String,
        arguments: HashMap<String, Value>,
        children: Vec<Self>,
    },
    Module {
        name: String,
        #[serde(default)]
        data: String,
        #[serde(default)]
        arguments: HashMap<String, Value>,
        #[serde(default = "default_inline")]
        inline: bool,
    },
    Compound(Vec<Self>),
    Raw(String),
}

/// This is just a helper to ensure that omitted "inline" fields
/// default to true.
fn default_inline() -> bool {
    true
}

macro_rules! raw {
    ($expr:expr) => {
        json!({
            "name": "raw",
            "data": $expr
        })
    }
}

macro_rules! inline_content {
    ($expr:expr) => {
        json!({
            "name": "inline_content",
            "data": $expr
        })
    }
}

macro_rules! block_content {
    ($expr:expr) => {
        json!({
            "name": "block_content",
            "data": $expr
        })
    }
}

macro_rules! dynamic_content {
    ($cond:expr, $expr:expr) => {
        if $cond {
            block_content!($expr)
        } else {
            inline_content!($expr)
        }
    };
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let Some(action) = args.get(0) else {
        eprintln!("No action was provided.");
        return;
    };

    match action.as_str() {
        "manifest" => print!("{}", &manifest()),
        "transform" => {
            let from = args.get(1).unwrap();
            let format = args.get(2).unwrap();

            if "html" != format {
                eprintln!("Output format not supported");
                return;
            }

            print!("{}", transform(from));
        }
        other => eprintln!("Invalid action '{other}'"),
    }
}

fn transform(from: &str) -> String {
    let input: JsonEntry = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        from_str(&buffer).unwrap()
    };

    match from {
        "__bold" => transform_tag(input, "strong", true),
        "__italic" => transform_tag(input, "em", true),
        "__superscript" => transform_tag(input, "sup", true),
        "__subscript" => transform_tag(input, "sub", true),
        "__underlined" => transform_tag(input, "u", true),
        "__verbatim" => transform_tag(input, ("code", "pre"), false),
        "__strikethrough" => transform_tag(input, "del", true),
        "__paragraph" => transform_tag(input, "p", true),
        "__math" => transform_math(to_value(input).unwrap()),
        "__document" => transform_document(to_value(input).unwrap()),
        "__text" => escape_text(to_value(input).unwrap()),
        "__heading" => transform_heading(to_value(input).unwrap()),
        "__error" => transform_error(to_value(input).unwrap()),
        _ => panic!("element not supported"),
    }
}

fn transform_document(mut doc: Value) -> String {
    let mut result = vec![raw!(
        r#"
<!DOCTYPE html>
<html>
<head>
<title>Document</title>
<meta charset="UTF-8">
"#
    )];

    // Add imports
    let mut imports = {
        let var = env::var("imports").unwrap_or("[]".to_string());
        serde_json::from_str(&var).unwrap()
    };
    result.append(&mut imports);

    result.push(raw!("<style>"));
    result.push(raw!(include_str!("templates/html.css")));
    result.push(raw!(
        r#"
</style>
</head>
<body>
<article>
"#
    ));

    if let Some(children) = doc.get_mut("children") {
        if let Value::Array(ref mut children) = children {
            result.append(children);
        } else {
            unreachable!("Children is not a list");
        }
    }

    result.push(raw!("</article></body></html>"));

    serde_json::to_string(&result).unwrap()
}

fn transform_heading(heading: Value) -> String {
    let mut result = String::new();
    result.push('[');

    let Value::String(s) = &heading["arguments"]["level"] else {
        panic!();
    };
    let level = s.parse::<u8>().unwrap().clamp(1, 6);

    write!(result, "{},", raw!(format!("<h{level}>"))).unwrap();

    if let Value::Array(children) = &heading["children"] {
        for child in children {
            result.push_str(&serde_json::to_string(child).unwrap());
            result.push(',');
        }
    }

    write!(result, "{}", raw!(format!("</h{level}>"))).unwrap();
    result.push(']');

    result
}

fn transform_error(error: Value) -> String {
    let mut result = String::new();
    result.push('[');

    let Value::String(source) = &error["arguments"]["source"] else {
        panic!();
    };
    let Value::String(input) = &error["arguments"]["input"] else {
        panic!();
    };
    let Value::String(err) = &error["data"] else {
        panic!();
    };

    // TODO: Maybe make these errors look better. Be careful though, see notes in API, don't use
    //   calls to other modules that may fail. I have taken care to not use __text but rather just
    //   entered the text myself, because if I used __text and that failed, it would lead to
    //   infinite recursion, which is bad
    write!(result, "{},", raw!(r#"<span style="display: inline-block; background:#ffebeb; padding: 0.5rem; color: black; border-radius: 0.3rem; box-shadow: 0 0 2px #0000003b;">"#)).unwrap();

    let data = escape(format!("Error originating from {source}: {err} on input {input}").as_str());
    write!(result, "{},", raw!(data)).unwrap();

    write!(result, "{}", raw!("</span>")).unwrap();

    result.push(']');

    result
}

fn escape_text(module: Value) -> String {
    if let Value::String(s) = &module["data"] {
        let s = escape(s);
        format!("[{}]", raw!(s).to_string())
    } else {
        panic!("Malformed text module");
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

trait HtmlTag {
    fn inline(&self) -> &str;
    fn multiline(&self) -> &str;

    fn dynamic(&self, inline: bool) -> &str {
        if inline {
            self.inline()
        } else {
            self.multiline()
        }
    }
}

impl HtmlTag for &str {
    fn inline(&self) -> &str {
        self
    }

    fn multiline(&self) -> &str {
        self
    }
}

impl HtmlTag for (&str, &str) {
    fn inline(&self) -> &str {
        self.0
    }

    fn multiline(&self) -> &str {
        self.1
    }
}

fn transform_tag<T: HtmlTag>(node: JsonEntry, html_tag: T, reparse: bool) -> String {
    let mut result: Vec<Value> = vec![];

    match node {
        JsonEntry::ParentNode { children, .. } => {
            result.push(Value::from(format!("<{}>", html_tag.inline())));
            result.extend(children.into_iter().map(|x| to_value(x).unwrap()));
            result.push(Value::from(format!("</{}>", html_tag.inline())));
        }
        JsonEntry::Module { data, inline, .. } => {
            result.push(Value::from(format!("<{}>", html_tag.dynamic(inline))));
            if reparse {
                result.push(dynamic_content!(inline, data));
            } else {
                result.push(json!({"name": "__text", "data": data}));
            }
            result.push(Value::from(format!("</{}>", html_tag.dynamic(inline))));
        }
        _ => {}
    }
    serde_json::to_string(&result).unwrap()
}

fn transform_math(node: Value) -> String {
    // We know that the math tag is a non-recursively parsed tag, which means that it may only
    // contain __text and modules. For now, we collect all __text nodes and
    if let Value::Array(children) = &node["children"] {
        let mut content = String::new();
        for child in children {
            let name = child["name"].as_str().unwrap();
            if name == "__text" {
                content.push_str(child["data"].as_str().unwrap());
            } else {
                eprintln!("Modules are not allowed in math tags; found module {name}");
            }
        }
        if content.is_empty() {
            format!("{}", json!([]))
        } else {
            format!(
                "{}",
                json!([
                    {
                      "name": "math",
                      "data": content,
                      "arguments": {},
                      "inline": true
                    }
                ])
            )
        }
    } else {
        eprintln!("Unexpected __math structure");
        "".to_string()
    }
}

fn manifest() -> String {
    serde_json::to_string(&json!(
        {
            "version": "0.1",
            "name": "html",
            "description": "This packages provides HTML support for the basic Modmark features.",
            "transforms": [
                {
                    "from": "__bold",
                    "to": ["html"],
                    "arguments": [],
                    "type": "any"
                },
                {
                    "from": "__italic",
                    "to": ["html"],
                    "arguments": [],
                    "type": "any"
                },
                {
                    "from": "__superscript",
                    "to": ["html"],
                    "arguments": [],
                    "type": "any"
                },
                {
                    "from": "__subscript",
                    "to": ["html"],
                    "arguments": [],
                    "type": "any"
                },
                {
                    "from": "__strikethrough",
                    "to": ["html"],
                    "arguments": [],
                    "type": "any"
                },
                {
                    "from": "__underlined",
                    "to": ["html"],
                    "arguments": [],
                    "type": "any"
                },
                {
                    "from": "__verbatim",
                    "to": ["html"],
                    "arguments": [],
                    "type": "any"
                },
                {
                    "from": "__document",
                    "to": ["html"],
                    "arguments": [],
                    "type": "parent",
                    "variables": {
                        "imports": {"type": "set", "access": "read"}
                    },
                },
                {
                    "from": "__text",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__math",
                    "to": ["html"],
                    "arguments": [],
                    "evaluate-before-children": true,
                    "type": "parent"
                },
                {
                    "from": "__paragraph",
                    "to": ["html"],
                    "arguments": [],
                    "type": "parent"
                },
                {
                    "from": "__error",
                    "to": ["html"],
                    "arguments": [
                        {
                            "name":"source",
                            "description":"Source for the error",
                            "default":"<unknown>"
                        },
                        {
                            "name":"target",
                            "description":"Target for the error",
                            "default":"<unknown>"
                        },
                        {
                            "name":"input",
                            "description":"Input for the error",
                            "default":"<unknown>"
                        },
                    ],
                },
                {
                    "from": "__heading",
                    "to": ["html"],
                    "arguments": [
                        {
                            "name": "level",
                            "description": "The level of the heading",
                            "default": "1"
                        }
                    ],
                    "type": "parent"
                }
            ]
        }
    ))
    .unwrap()
}
