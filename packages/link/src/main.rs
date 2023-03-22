use std::env;
use std::io::{self, Read};

use serde_json::{json, Value};

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
            "version": "0.2",
            "description": "This package supports [link] modules",
            "transforms": [
                {
                    "from": "link",
                    "to": ["html", "latex"],
                    "arguments": [
                        {"name": "label", "default": "", "description": "Label for link"}
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
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_link(to: &str) {
    match to {
        "html" => {
            let input: Value = {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();
                serde_json::from_str(&buffer).unwrap()
            };

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
            let input: Value = {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();
                serde_json::from_str(&buffer).unwrap()
            };

            let label = input["arguments"]
                .get("label")
                .map(|val| val.as_str().unwrap())
                .unwrap_or_else(|| "");
            let link = input["data"].as_str().unwrap();

            let text = if label.is_empty() { link } else { label };
            let data = format!(r#"\href{{{}}}{{{}}}"#, link, text);

            let output = json!([
                {"name": "raw", "data": data},
            ]);
            print!("{output}");
        }
        other => {
            eprintln!("Cannot convert link to {other}");
        }
    }
}
