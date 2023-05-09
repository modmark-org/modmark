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
        json!(
            {
            "name": "list",
            "version": "0.1",
            "description": "This package supports [list] modules. Lists can use alpha, decimal, romans and bulletpoints (+, *, or -). When creating a numbered list items can be 1. 1) or (1). First ordered item is used as a starting point then increments.",
            "transforms": [
                {
                    "from": "list",
                    "to": ["html", "latex"],
                    "arguments": [
                        {
                            "name": "indent",
                            "default": 4,
                            "description": "Number of spaces needed for each level of indent when writing the list.",
                            "type": "unsigned_integer"
                        }
                    ],
                    "variables": {
                        "imports": {"type": "set", "access": "add"}
                    },
                    "unknown-content": true
                },
            ]
            }
        )
    )
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
            let indent = input["arguments"]["indent"].as_u64().unwrap();

            if let Ok(list) = List::from_str(body, indent) {
                print!("{}", list.to_html())
            } else {
                eprintln!("Module block does not start with a list")
            }
        }
        "latex" => {
            let input: Value = {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();
                serde_json::from_str(&buffer).unwrap()
            };

            let body = input["data"].as_str().unwrap();
            let indent = input["arguments"]["indent"].as_u64().unwrap();

            if let Ok(list) = List::from_str(body, indent) {
                print!("{}", list.to_latex())
            } else {
                eprintln!("Module block does not start with a list")
            }
        }
        other => {
            eprintln!("Cannot convert list to {other}");
        }
    }
}
