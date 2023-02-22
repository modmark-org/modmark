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
        "__bold" => transform_tag(input, "strong"),
        "__italic" => transform_tag(input, "em"),
        "__superscript" => transform_tag(input, "sup"),
        "__subscript" => transform_tag(input, "sub"),
        "__underlined" => transform_tag(input, "u"),
        "__strikethrough" => transform_tag(input, "del"),
        "__paragraph" => transform_tag(input, "p"),
        "__document" => transform_document(input),
        "__text" => escape_text(input),
        "__heading" => transform_heading(input),
        _ => panic!("element not supported"),
    }
}

fn transform_tag(node: Value, latex_function: &str) -> String {
    unimplemented!("transform_tag is not implemented yet!");
}

fn transform_heading(heading: Value) -> String {
    unimplemented!("transform_heading is not implemented yet!");
}

fn transform_document(doc: Value) -> String {
    unimplemented!("transform_document is not implemented yet!");
}

fn escape_text(module: Value) -> String {
    if let Value::String(s) = &module["data"] {
        let s = s
            .replace('#', r"\#")
            .replace('$', r"\$")
            .replace('%', r"\%")
            .replace('&', r"\&")
            .replace('\\', r"\textbacklash{}")
            .replace('^', r"\textasciicircum{}")
            .replace('_', r"\_")
            .replace('{', r"\{")
            .replace('}', r"\}")
            .replace('~', r"\textasciitilde{}");
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