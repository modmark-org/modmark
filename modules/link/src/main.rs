use std::env;
use std::fmt::Write;
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
            "name": "Standard link package",
            "version": "0.1",
            "description": "This package supports [link] modules",
            "transforms": [
                {
                    "from": "link",
                    "to": ["html"],
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

fn transform(from: &String, to: &String) {
    match from.as_str() {
        "link" => transform_link(to),
        other => {
            eprintln!("Package does not support {other}");
            return;
        }
    }
}

fn transform_link(to: &String) {
    match to.as_str() {
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

            let output = if label == "" {
                format!(r#"{{"name": "raw", "data": "<a href='{link}'>{link}</a>"}}"#)
            } else {
                format!(r#"{{"name": "raw", "data": "<a href='{link}'>{label}</a>"}}"#)
            };

            print!("[{output}]");
        }
        other => {
            eprintln!("Cannot convert table to {other}");
            return;
        }
    }
}
