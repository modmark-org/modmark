use std::env;
use std::io::{self, Read};
use std::fs;
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
    print!("{}", serde_json::to_string(&json!(
        {
        "name": "textfile",
        "version": "0.1",
        "description": "This package tests file access",
        "transforms": [
            {
                "from": "file",
                "to": ["html"],
                "arguments": [],
            }
        ]
        }
    ))
    .unwrap());
}

fn transform(from: &str, to: &str) {
    match from {
        "file" => transform_file(to),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_file(to: &str) {
    match to {
        "html" => {

            let input: Value = {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();
                serde_json::from_str(&buffer).unwrap()
            };

            let path = input["data"].as_str().unwrap();
            match fs::read_to_string(path) {
                Ok(contents) => {
                    let html = format!("<p>{contents}</p>");
                    let json = json!({"name": "raw", "data": html}).to_string();
                    print!("[{json}]");
                },
                _ => {
                    let json = json!({"name": "raw", "data": ""}).to_string();
                    print!("[{json}]");
                    eprintln!("No file was found at {path}")
                }
            }
        }
        other => {
            eprintln!("Cannot convert file to {other}");
        }
    }
}
