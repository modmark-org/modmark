use std::env;
use std::io::{self, Read};

use serde_json::{json, Value};

macro_rules! import {
    ($e:expr) => {json!({"name": "set-add", "arguments": {"name": "imports"}, "data": $e})}
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
                        "imports": {"type": "set", "access": "add"}
                    }
                },
                {
                    "from": "label",
                    "to": ["html", "latex"],
                    "arguments": [],
                },
                {
                    "from": "reference",
                    "to": ["html", "latex"],
                    "arguments": [
                        {"name": "display", "default": "", "description": "Displayed label for reference (Only HTML)"}
                    ],
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
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_link(to: &str) {
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };

    match to {
        "html" => {
            let label = input["arguments"]
                .get("label")
                .map(|val| val.as_str().unwrap())
                .unwrap_or_else(|| "");

            let link = input["data"].as_str().unwrap();
            let escaped_link = link.replace('"', "%22");

            let link_tag = format!(r#"<a href="{escaped_link}">"#);
            let text = if label.is_empty() { link } else { label };

            let output = json!([
                {"name": "raw", "data": link_tag},
                {"name": "inline_content", "data": text},
                {"name": "raw", "data": "</a>"}
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
            let prefix = format!(r#"\href{{{}}}{{"#, link);

            let output = json!([
                {"name": "raw", "data": prefix},
                {"name": "inline_content", "data": text},
                {"name": "raw", "data": "}"},
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

            let output = json!([
                {"name": "raw", "data": label_tag},
                {"name": "raw", "data": "</span>"},
            ]);

            print!("{output}");
        }
        "latex" => {
            let label = input["data"].as_str().unwrap();
            let escaped_label = label.replace('"', "%22");

            let label_tag = format!(r#"\label{{{}}}"#, escaped_label);

            let output = json!([
                {"name": "raw", "data": label_tag},
            ]);

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
            let mut escaped_label = label.replace('"', "%22");
            escaped_label.insert(0, '#');

            let display = input["arguments"]
                .get("display")
                .map(|val| val.as_str().unwrap())
                .unwrap_or_else(|| "");

            let label_tag = format!(r#"<a href="{escaped_label}">"#);

            let output = json!([
                {"name": "raw", "data": label_tag},
                {"name": "raw", "data": display},
                {"name": "raw", "data":  "</a>"},
            ]);

            print!("{output}");
        }
        "latex" => {
            let label = input["data"].as_str().unwrap();
            let escaped_label = label.replace('"', "%22");

            let label_tag = format!(r#"\ref{{{}}}"#, escaped_label);

            let output = json!([
                {"name": "raw", "data": label_tag},
            ]);

            print!("{output}");
        }
        other => {
            eprintln!("Cannot convert ref to {other}");
        }
    }
}
