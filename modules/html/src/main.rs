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
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        from_str(&buffer).unwrap()
    };

    match from {
        "__bold" => transform_tag(input, "strong"),
        "__italic" => transform_tag(input, "em"),
        "__superscript" => transform_tag(input, "sup"),
        "__subscript" => transform_tag(input, "sub"),
        "__underlined" => transform_tag(input, "u"),
        "__strikethrough" => transform_tag(input, "del"),
        "__paragraph" => transform_tag(input, "p"),
        "__document" => transform_document(input),
        "__text" => escape_text(input),
        _ => unreachable!(),
    }
}

fn transform_document(doc: Value) -> String {
    let mut result = String::new();
    result.push('[');

    write!(
        result,
        r#"{{"name": "raw", "data": "<html><head><title>Document</title></head><body>"}},"#
    )
    .unwrap();

    if let Value::Array(children) = &doc["children"] {
        for child in children {
            result.push_str(&serde_json::to_string(child).unwrap());
            result.push(',');
        }
    }

    write!(result, r#"{{"name": "raw", "data": "</body>"}}"#).unwrap();
    result.push(']');

    result
}

fn escape_text(module: Value) -> String {
    if let Value::String(s) = &module["data"] {
        let s = s
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;");
        format!(r#"[{{"name": "raw", "data": "{s}"}}]"#)
    } else {
        panic!("Malformed text module");
    }
}

fn transform_tag(node: Value, html_tag: &str) -> String {
    let mut result = String::new();
    result.push('[');
    write!(result, r#"{{"name": "raw", "data": "<{html_tag}>" }},"#).unwrap();

    if let Value::Array(children) = &node["children"] {
        for child in children {
            result.push_str(&serde_json::to_string(child).unwrap());
            result.push(',');
        }
    }

    write!(result, r#"{{"name": "raw", "data": "</{html_tag}>" }}"#).unwrap();
    result.push(']');

    result
}

fn manifest() -> String {
    serde_json::to_string(&json!(
        {
            "version": "0.1",
            "name": "HTML",
            "description": "This packages provides HTML support for the basic Modmark features.",
            "transforms": [
                {
                    "from": "__bold",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__italic",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__superscript",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__subscript",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__underlined",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__document",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__text",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__paragraph",
                    "to": ["html"],
                    "arguments": [],
                },

            ]
        }
    ))
    .unwrap()
}
