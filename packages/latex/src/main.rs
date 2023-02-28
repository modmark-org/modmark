use serde_json::{from_str, json, Value};
use std::{
    env,
    fmt::Write,
    io::{self, Read},
};

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
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        from_str(&buffer).unwrap()
    };

    match from {
        "__bold" => transform_tag(input, "textbf"),
        "__italic" => transform_tag(input, "emph"),
        "__superscript" => transform_tag(input, "textsuperscript"),
        "__subscript" => transform_tag(input, "textsubscript"),
        "__underlined" => transform_tag(input, "underline"),
        "__strikethrough" => transform_tag(input, "sout"), //fixme: needs a package to use
        "__verbatim" => transform_block(input, "verbatim"),
        "__paragraph" => transform_paragraph(input),
        "__document" => transform_block(input, "document"),
        "__text" => escape_text(input),
        "__heading" => transform_heading(input),
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

fn transform_tag(node: Value, latex_function: &str) -> String {
    let mut result = String::new();
    result.push('[');
    write!(result, r#"{{"name": "raw", "data": "\\{latex_function}{{"}},"#,).unwrap();
    if let Value::Array(children) = &node["children"] {
        for child in children {
            result.push_str(&serde_json::to_string(child).unwrap());
            result.push(',');
        }
    }
    write!(result, r#"{{"name": "raw", "data": "}}"}}"#,).unwrap();
    result.push(']');

    result
}

fn transform_heading(heading: Value) -> String {
    let mut result = String::new();
    result.push('[');

    let Value::String(s) = &heading["arguments"]["level"] else {
        panic!();
    };
    let level = s.parse::<u8>().unwrap().clamp(1, 6);
    let mut subs = String::new();
    if level > 1 {
        subs.push_str(&"sub".repeat((level - 1) as usize));
    }
    

    write!(result, r#"{{"name": "raw", "data": "\n\\{subs}section{{"}},"#,).unwrap();
    if let Value::Array(children) = &heading["children"] {
        for child in children {
            result.push_str(&serde_json::to_string(child).unwrap());
            result.push(',');
        }
    }
    write!(result, r#"{{"name": "raw", "data": "}}\n"}}"#,).unwrap();
    result.push(']');

    result
}

fn transform_block(doc: Value, tag: &str) -> String {
    let mut result = String::new();
    result.push('[');
    write!(result, r#"{{"name": "raw", "data": "\\begin{{{tag}}}\n"}},"#,).unwrap();
    if let Value::Array(children) = &doc["children"] {
        for child in children {
            result.push_str(&serde_json::to_string(child).unwrap());
            result.push(',');
        }
    }
    write!(result, r#"{{"name": "raw", "data": "\n\\end{{{tag}}}"}}"#,).unwrap();
    result.push(']');

    result
}


fn escape_text(module: Value) -> String {
    if let Value::String(s) = &module["data"] {
        let s = s.split('\\').map(|t| t.replace('{', r"\\{").replace('}', r"\\}")).collect::<Vec<String>>().join(r"\\textbacklash{}")
            .replace('#', r"\\#")
            .replace('$', r"\\$")
            .replace('%', r"\\%")
            .replace('&', r"\\&")
            .replace('_', r"\\_")
            .replace('\n', " ")
            .replace('~', r"\\textasciitilde{}")
            .replace('^', r"\\textasciicircum{}");
        format!(r#"[{{"name": "raw", "data":"{s}"}}]"#)
    } else {
        panic!("Malformed text module");
    }
}

fn manifest() -> String {
    serde_json::to_string(&json!(
        {
            "version": "0.1",
            "name": "Latex",
            "description": "This packages provides Latex support for the basic Modmark features.",
            "transforms": [
                {
                    "from": "__bold",
                    "to": ["latex"],
                    "arguments": [],
                },
                {
                    "from": "__italic",
                    "to": ["latex"],
                    "arguments": [],
                },
                {
                    "from": "__superscript",
                    "to": ["latex"],
                    "arguments": [],
                },
                {
                    "from": "__subscript",
                    "to": ["latex"],
                    "arguments": [],
                },
                {
                    "from": "__strikethrough",
                    "to": ["latex"],
                    "arguments": [],
                },
                {
                    "from": "__underlined",
                    "to": ["latex"],
                    "arguments": [],
                },
                {
                    "from": "__document",
                    "to": ["latex"],
                    "arguments": [],
                },
                {
                    "from": "__text",
                    "to": ["latex"],
                    "arguments": [],
                },
                {
                    "from": "__paragraph",
                    "to": ["latex"],
                    "arguments": [],
                },
                {
                    "from": "__verbatim",
                    "to": ["latex"],
                    "arguments": [],
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
                },

            ]
        }
    ))
    .unwrap()
}