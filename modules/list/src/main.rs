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
            "name": "Standard list package",
            "version": "0.1",
            "description": "This package supports [list] modules",
            "transforms": [
                {
                    "from": "list",
                    "to": ["html"],
                    "arguments": [
                        {"name": "bullet_points", "default": "", "description": "Label for link"}
                    ],
                },
                {
                    "from": "enumerate"
                    "to"
                }
            ]
            }
        ))
        .unwrap()
    );
}

fn transform(from: &String, to: &String) {
    match from.as_str() {
        "link" => transform_table(to),
        other => {
            eprintln!("Package does not support {other}");
            return;
        }
    }
}

fn transform_table(to: &String) {
    match to.as_str() {
        "html" => {
            let input: Value = {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();
                serde_json::from_str(&buffer).unwrap()
            };

            let label = input["arguments"]
                .get("label")
                .map(|val| serde_json::to_string(val).unwrap())
                .unwrap_or_else(|| "".to_string());

            let body = input["data"].as_str().unwrap();

            let output = if label == "" {
                format!(r#"{{"name": "raw", "data": "<a href=\"{body}\">{body}</a>"}}"#)
            } else {
                format!(r#"{{"name": "raw", "data": "<a href=\"{body}\">{label}</a>"}}"#)
            };

            print!("{output}");
        }
        other => {
            eprintln!("Cannot convert table to {other}");
            return;
        }
    }
}
