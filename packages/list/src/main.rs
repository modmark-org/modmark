use std::env;
use std::io::{self, Read};

use list::List;
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
            "name": "list",
            "version": "0.1",
            "description": "This package supports [list] modules",
            "transforms": [
                {
                    "from": "list",
                    "to": ["html"],
                    "arguments": [],
                },
            ]
            }
        ))
        .unwrap()
    );
}

fn transform(from: &str, to: &str) {
    match from {
        "list" => transform_list(to),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_list(to: &str) {
    match to {
        "html" => {
            let input: Value = {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();
                serde_json::from_str(&buffer).unwrap()
            };

            let body = input["data"].as_str().unwrap();

            if let Ok(list) = body.parse::<List>() {
                print!("{}", list.to_html())
            } else {
                eprintln!("Module block does not start with a list")
            }
        }
        other => {
            eprintln!("Cannot convert list to {other}");
        }
    }
}
