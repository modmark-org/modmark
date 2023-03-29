use std::env;
use std::io::{self, Read};
use std::fs;
use serde_json::{json, Value};
use base64::{Engine as _, engine::general_purpose};

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
        "name": "files",
        "version": "0.1",
        "description": "This package tests file access",
        "transforms": [
            {
                "from": "textfile",
                "to": ["html"],
                "arguments": [],
            },
            {
                "from": "image",
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
        "textfile" => transform_text(to),
        "image" => transform_image(to),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_text(to: &str) {
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

fn transform_image(to: &str) {
    match to {
        "html" => {

            let input: Value = {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();
                serde_json::from_str(&buffer).unwrap()
            };

            let path = input["data"].as_str().unwrap();
            match fs::read(path) {
                Ok(contents) => {
                    let encoded: String = general_purpose::STANDARD_NO_PAD.encode(contents);
                    let html = format!("<img src=\"data:image/png;base64, {encoded} \"/>");
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
