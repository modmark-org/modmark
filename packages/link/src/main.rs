use std::env;
use std::io::{self, Read};

use serde_json::{json, Value};

macro_rules! import {
    ($e:expr) => {json!({"name": "set-add", "arguments": {"name": "imports"}, "data": $e})}
}

macro_rules! inline_target {
    ($e:expr) => {json!({"name": "set-add", "arguments": {"name": "inline_targets"}, "data": $e})}
}

macro_rules! module {
    ($name:expr, $data:expr $(,$($args:tt)*)?) => {json!({"name": $name $(,"arguments":$($args)*)*, "data": $data})}
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let action = &args[0];
    match action.as_str() {
        "manifest" => manifest(),
        "transform" => transform(&args[1], &args[2]),
        other => {
            eprintln!("Invalid action {other}")
        }
    }
}

fn manifest() {
    print!(
        "{}",
        serde_json::to_string(&json!(
            {
            "name": "link",
            "version": "0.3",
            "description": "This package supports [link] modules. It can link internally with labels and references, or externally with URLs.",
            "transforms": [
                {
                    "from": "link",
                    "to": ["html", "latex"],
                    "arguments": [
                        {"name": "label", "default": "", "description": "Label for link"}
                    ],
                    "variables": {
                        "imports": {"type": "set", "access": "add"},
                        "inline_targets": {"type": "set", "access": "read"}
                    },
                    "description": "Inserts a link to an URL or to a target in the document"
                },
                {
                    "from": "label",
                    "to": ["html", "latex"],
                    "arguments": [],
                    "variables": {
                        "structure": {"type": "list", "access": "push"}
                    },
                },
                {
                    "from": "reference",
                    "to": ["html", "latex"],
                    "arguments": [],
                    "variables": {
                        "structure": {"type": "list", "access": "read"}
                    },
                },
                {
                    "from": "target",
                    "to": ["html", "latex"],
                    "arguments": [
                        {"name": "name", "type": "string", "description": "The name used to refer to a target later on"}
                    ],
                    "description": "Marks the body as a 'target', which later can be linked to using [link]",
                    "variables": {
                        "inline_targets": {"type": "set", "access": "add"}
                    },
                    "type": "inline-module"
                }
            ]
            }
        ))
        .unwrap()
    );
}

fn transform(from: &str, to: &str) {
    match from {
        "link" => transform_link(to),
        "label" => transform_label(to),
        "reference" => transform_reference(to),
        "target" => transform_target(to),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_target(to: &str) {
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };
    let name = input["arguments"]["name"].as_str().unwrap();
    let body = input["data"].as_str().unwrap();

    let result = if to == "latex" {
        let mut res = vec![];
        res.push(import!(r"\usepackage[hidelinks]{hyperref}"));
        res.push(inline_target!(name));
        res.push(Value::String(format!(
            "\\hypertarget{{inlinetarget{}}}{{",
            name
        )));
        res.push(module!("inline_content", body));
        res.push(Value::from("}"));
        res
    } else if to == "html" {
        let mut res = vec![];
        res.push(inline_target!(name));
        res.push(Value::String(format!("<span id=\"inlinetarget{}\">", name)));
        res.push(module!("inline_content", body));
        res.push(Value::from("</span>"));
        res
    } else {
        panic!("[target] only supports HTML and LaTeX");
    };
    println!("{}", Value::Array(result));
}

fn transform_link(to: &str) {
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };
    let targets: Vec<String> =
        serde_json::from_str(&env::var("inline_targets").unwrap_or("[]".to_string())).unwrap();
    let data = input["data"].as_str().unwrap();
    let is_target = targets.iter().any(|t| t == data);

    match to {
        "html" => {
            let label = input["arguments"]
                .get("label")
                .map(|val| val.as_str().unwrap())
                .unwrap_or_else(|| "");

            let link = input["data"].as_str().unwrap();
            let actual_link = if is_target {
                format!("#inlinetarget{}", link)
            } else {
                link.replace('"', "%22")
            };

            let link_tag = format!(r#"<a href="{actual_link}">"#);
            let text = if label.is_empty() { link } else { label };

            let output = json!([
                link_tag,
                {"name": "inline_content", "data": text},
                "</a>",
            ]);
            print!("{output}");
        }
        "latex" => {
            let label = input["arguments"]
                .get("label")
                .map(|val| val.as_str().unwrap())
                .unwrap_or_else(|| "");
            let link = input["data"].as_str().unwrap();

            let text = if label.is_empty() { link } else { label };
            let prefix = if is_target {
                format!(r#"\hyperlink{{inlinetarget{}}}{{"#, link)
            } else {
                format!(r#"\href{{{}}}{{"#, link)
            };

            let output = json!([
                prefix,
                {"name": "inline_content", "data": text},
                "}",
                import![r"\usepackage[hidelinks]{hyperref}"]
            ]);
            print!("{output}");
        }
        other => {
            eprintln!("Cannot convert link to {other}");
        }
    }
}

fn transform_label(to: &str) {
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };

    match to {
        "html" => {
            let label = input["data"].as_str().unwrap();
            let escaped_label = label.replace('"', "%22");
            let label_tag = format!(r#"<span id="{escaped_label}">"#);
            let structure_data = json!({"element": "label", "key": label}).to_string();
            let mut json = vec![];

            json.push(json!(label_tag));
            json.push(json!("</span>"));
            json.push(json!(
                {
                    "name": "list-push",
                    "arguments":{"name": "structure"},
                    "data": structure_data,
                }
            ));

            print!("{}", serde_json::to_string(&json).unwrap());
        }
        "latex" => {
            let label = input["data"].as_str().unwrap();
            let escaped_label = label.replace('"', "%22");

            let label_tag = format!(r#"\label{{{}}}"#, escaped_label);

            let output = json!(label_tag);

            print!("{output}");
        }
        other => {
            eprintln!("Cannot convert label to {other}");
        }
    }
}

fn transform_reference(to: &str) {
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };

    match to {
        "html" => {
            let label = input["data"].as_str().unwrap();

            let output = json!([
                "<a href=\"#",
                {"name": "inline_content", "data": format!("[label-to-id]({label})")},
                "\">",
                {"name": "inline_content", "data": format!("[element-number]({label})")},
                "</a>",
            ]);

            print!("{output}");
        }
        "latex" => {
            let label = input["data"].as_str().unwrap();
            let escaped_label = label.replace('"', "%22");

            let label_tag = format!(r#"\ref{{{}}}"#, escaped_label);

            let output = json!(label_tag);

            print!("{output}");
        }
        other => {
            eprintln!("Cannot convert ref to {other}");
        }
    }
}
