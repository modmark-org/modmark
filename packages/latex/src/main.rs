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

macro_rules! import {
    ($e:expr) => {json!({"name": "set-add", "arguments": {"name": "imports"}, "data": $e})}
}

macro_rules! single_import {
    ($e:expr) => {
        vec![import![$e]]
    };
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

            if "latex" != format {
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
        "__bold" => transform_tag(input, "textbf", true),
        "__italic" => transform_tag(input, "textit", true),
        "__superscript" => transform_tag(input, "textsuperscript", true),
        "__subscript" => transform_tag(input, "textsubscript", true),
        "__underlined" => transform_tag(input, "underline", true),
        "__strikethrough" => transform_tag(input, "sout", true),
        "__verbatim" => transform_verbatim(input),
        "__paragraph" => transform_paragraph(to_value(input).unwrap()),
        "__document" => transform_document(to_value(input).unwrap()),
        "__math" => transform_math(to_value(input).unwrap()),
        "__text" => escape_text(to_value(input).unwrap()),
        "__heading" => transform_heading(to_value(input).unwrap()),
        _ => panic!("element not supported"),
    }
}

fn transform_paragraph(paragraph: Value) -> String {
    let mut result = String::new();
    result.push('[');
    write!(result, r#"{{"name": "raw", "data": "\n"}},"#,).unwrap();
    if let Value::Array(children) = &paragraph["children"] {
        for child in children {
            result.push_str(&serde_json::to_string(child).unwrap());
            result.push(',');
        }
    }
    write!(result, r#"{{"name": "raw", "data": "\n"}}"#,).unwrap();
    result.push(']');

    result
}

fn transform_tag(node: JsonEntry, latex_function: &str, reparse: bool) -> String {
    let mut result: Vec<Value> = vec![];
    result.push(Value::from(format!("\\{latex_function}{{")));

    match node {
        JsonEntry::ParentNode { children, .. } => {
            result.extend(children.into_iter().map(|x| to_value(x).unwrap()));
        }
        JsonEntry::Module { data, inline, .. } => {
            if reparse {
                result.push(dynamic_content!(inline, data));
            } else {
                result.push(json!({"name": "__text", "data": data}));
            }
        }
        _ => {}
    }

    result.push(Value::from("}"));
    result.append(&mut get_imports_for_tag(latex_function));
    serde_json::to_string(&result).unwrap()
}

fn get_imports_for_tag(latex_function: &str) -> Vec<Value> {
    // Here we can define imports for tags passed to transform_tag
    // Use single_import! with the import text to add one import, or if you need multiple, use
    // vec![import!["\usepackage{...}"], import!["\usepackage{...}"]]
    match latex_function {
        "sout" => single_import![r"\usepackage[normalem]{ulem}"],
        _ => vec![],
    }
}

fn transform_verbatim(node: JsonEntry) -> String {
    let mut result: Vec<Value> = vec![];

    match node {
        JsonEntry::Module { data, inline, .. } => {
            if inline {
                result.push(Value::from("\\verb|"));
                result.push(Value::from(data.replace('|', r"\|")));
                result.push(Value::from("|"));
            } else {
                result.push(Value::from("\n\\begin{verbatim}\n"));
                result.push(Value::from(data));
                result.push(Value::from("\n\\end{verbatim}\n"));
            }
        }
        JsonEntry::ParentNode { children, .. } => {
            result.push(Value::from("\\verb|"));
            for child in children {
                if let JsonEntry::Module {
                    ref name, ref data, ..
                } = child
                {
                    if name == "__text" {
                        result.push(Value::from(data.replace('|', r"\|")));
                    } else {
                        result.push(to_value(child).unwrap());
                    }
                } else {
                    result.push(to_value(child).unwrap());
                }
            }
            result.push(Value::from("|"));
        }
        _ => {}
    }
    serde_json::to_string(&result).unwrap()
}

fn transform_heading(heading: Value) -> String {
    let mut vec = vec![];
    
    let Value::String(s) = &heading["arguments"]["level"] else {
        panic!();
    };
    let level = s.parse::<u8>().unwrap();
    if level > 3 {
        eprintln!("Latex only supports headings up to level 3");
    }
    let clamped_level = level.clamp(1, 3); //latex only supports 1-3
    let mut subs = String::new();
    if clamped_level > 1 {
        subs.push_str(&"sub".repeat((clamped_level - 1) as usize));
    }

    let heading_style = env::var("heading_style").unwrap_or(String::new());

    if heading_style == "unnumbered" {
        vec.push(json!(format!("\n\\{subs}section*{{")));
    } else {
        vec.push(json!(format!("\n\\{subs}section{{")));
    }

    if let Value::Array(children) = &heading["children"] {
        for child in children {
            vec.push(child.clone());
        }
    }

    vec.push(json!("}\n"));

    if heading_style == "unnumbered" {
        vec.push(json!(format!("\\addcontentsline{{toc}}{{{subs}section}}{{")));
        if let Value::Array(children) = &heading["children"] {
            for child in children {
                vec.push(child.clone());
            }
        }
        vec.push(json!("}\n"));
    }

    serde_json::to_string(&vec).unwrap()
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
        String::new()
    }
}

fn transform_document(mut doc: Value) -> String {
    let mut result: Vec<Value> = vec![];

    result.push(Value::from(r"\documentclass{article}"));

    let imports_var = env::var("imports").unwrap_or("[]".to_string());
    let imports: Vec<String> = serde_json::from_str(&imports_var).unwrap();

    if !imports.is_empty() {
        result.push(Value::from("\n"));
        for import in imports {
            result.push(Value::from(format!("\n{import}")));
        }
    }

    result.push(Value::from("\n\n\\begin{document}\n\n"));

    if let Some(vec) = doc.get_mut("children").and_then(Value::as_array_mut) {
        result.append(vec);
    }
    result.push(Value::from("\n\n\\end{document}"));

    serde_json::to_string(&result).unwrap()
}

fn escape_text(module: Value) -> String {
    if let Value::String(s) = &module["data"] {
        let s = s
            .split('\\')
            .map(|t| t.replace('{', r"\{").replace('}', r"\}"))
            .collect::<Vec<String>>()
            .join(r"\textbackslash{}")
            .replace('#', r"\#")
            .replace('$', r"\$")
            .replace('%', r"\%")
            .replace('&', r"\&")
            .replace('_', r"\_")
            .replace('<', r"\textless{}")
            .replace('>', r"\textgreater{}")
            .replace('~', r"\textasciitilde{}")
            .replace('^', r"\textasciicircum{}");
        format!("{}", json! {[{"name":"raw","data":s}]})
    } else {
        panic!("Malformed text module");
    }
}

fn manifest() -> String {
    serde_json::to_string(&json!(
        {
            "version": "0.1",
            "name": "latex",
            "description": "This packages provides Latex support for the basic Modmark features.",
            "transforms": [
                {
                    "from": "__bold",
                    "to": ["latex"],
                    "arguments": [],
                    "type": "any"
                },
                {
                    "from": "__italic",
                    "to": ["latex"],
                    "arguments": [],
                    "type": "any"
                },
                {
                    "from": "__superscript",
                    "to": ["latex"],
                    "arguments": [],
                    "type": "any"
                },
                {
                    "from": "__subscript",
                    "to": ["latex"],
                    "arguments": [],
                    "type": "any"
                },
                {
                    "from": "__strikethrough",
                    "to": ["latex"],
                    "arguments": [],
                    "variables": {
                        "imports": {"type": "set", "access": "add"}
                    },
                    "type": "any"
                },
                {
                    "from": "__underlined",
                    "to": ["latex"],
                    "arguments": [],
                    "type": "any"
                },
                {
                    "from": "__math",
                    "to": ["latex"],
                    "arguments": [],
                    "evaluate-before-children": true,
                    "type": "parent"
                },
                {
                    "from": "__document",
                    "to": ["latex"],
                    "arguments": [],
                    "variables": {
                        "imports": {"type": "set", "access": "read"}
                    },
                    "type": "parent"
                },
                {
                    "from": "__text",
                    "to": ["latex"],
                    "arguments": []
                },
                {
                    "from": "__paragraph",
                    "to": ["latex"],
                    "arguments": [],
                    "type": "parent"
                },
                {
                    "from": "__verbatim",
                    "to": ["latex"],
                    "arguments": [],
                    "evaluate-before-children": true,
                    "type": "any"
                },
                {
                    "from": "__heading",
                    "to": ["latex"],
                    "arguments": [
                        {
                            "name": "level",
                            "description": "The level of the heading",
                            "default": "1"
                        }
                    ],
                    "type": "parent",
                    "variables": {
                        "heading_style": {"type": "const", "access": "read"},
                    },
                },

            ]
        }
    ))
    .unwrap()
}
