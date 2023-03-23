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
        "__italic" => transform_tag(input, "textit"),
        "__superscript" => transform_tag(input, "textsuperscript"),
        "__subscript" => transform_tag(input, "textsubscript"),
        "__underlined" => transform_tag(input, "underline"),
        "__strikethrough" => transform_tag(input, "sout"), //fixme: needs a package to use
        "__verbatim" => transform_verbatim(input),
        "__paragraph" => transform_paragraph(input),
        "__document" => transform_document(input),
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
    write!(
        result,
        r#"{{"name": "raw", "data": "\\{latex_function}{{"}},"#,
    )
    .unwrap();
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

fn transform_verbatim(text: Value) -> String {
    let mut result = String::new();
    result.push('[');
    write!(result, r#"{{"name": "raw", "data": "\\verb|"}},"#,).unwrap();
    if let Value::Array(children) = &text["children"] {
        for child in children {
            result.push_str(&serde_json::to_string(child).unwrap());
            result.push(',');
        }
    }
    write!(result, r#"{{"name": "raw", "data": "|"}}"#,).unwrap();
    result.push(']');

    result
}

fn transform_heading(heading: Value) -> String {
    let mut result = String::new();
    result.push('[');

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

    write!(
        result,
        r#"{{"name": "raw", "data": "\n\\{subs}section{{"}},"#,
    )
    .unwrap();
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

fn transform_document(doc: Value) -> String {
    let mut result = String::new();
    result.push('[');
    write!(result, r#"{{"name": "raw", "data": "\\documentclass{{article}}\n\n\\usepackage{{ulem}}\n\\usepackage[hidelinks]{{hyperref}}\n\\usepackage{{float}}\n\n\\begin{{document}}\n"}},"#,).unwrap();
    if let Value::Array(children) = &doc["children"] {
        for child in children {
            result.push_str(&serde_json::to_string(child).unwrap());
            result.push(',');
        }
    }
    write!(
        result,
        r#"{{"name": "raw", "data": "\n\\end{{document}}"}}"#,
    )
    .unwrap();
    result.push(']');

    result
}

fn escape_text(module: Value) -> String {
    if let Value::String(s) = &module["data"] {
        let s = s
            .split('\\')
            .map(|t| t.replace('{', r"\{").replace('}', r"\}"))
            .collect::<Vec<String>>()
            .join(r"\textbacklash{}")
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
