use std::env;
use std::fmt::Write;
use std::io::{self, Read};

use serde_json::{json, Value};
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{Color, ThemeSet};
use syntect::html::highlighted_html_for_string;

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
        "name": "Standard code package",
        "version": "0.1",
        "description": "This package provides syntax highlighting in [code] modules",
        "transforms": [
            {
                "from": "code",
                "to": ["html"],
                "arguments": [
                    {"name": "lang", "description": "The language to be highlighted"},
                    {"name": "fontsize", "default": "12", "description": "The size of the font"}
                ],
            }
        ]
        }
    ))
    .unwrap());
}

fn transform(from: &String, to: &String) {
    match from.as_str() {
        "code" => transform_code(to),
        other => {
            eprintln!("Package does not support {other}");
            return;
        }
    }
}

fn transform_code(to: &String) {
    match to.as_str() {
        "html" => {
            let input: Value = {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();
                serde_json::from_str(&buffer).unwrap()
            };

            let Value::String(lang) = &input["arguments"]["lang"] else {
                panic!("No lang argument was provided");
            };
            let Value::String(size) = &input["arguments"]["fontsize"] else {
                panic!("No fontsize argument was provided");
            };

            let code = input["data"].as_str().unwrap();

            let style = format!("style=\\\"font-size:{size}px\\\"");
            let ss = SyntaxSet::load_defaults_newlines();
            let ts = ThemeSet::load_defaults();
            let syntax = ss.find_syntax_by_extension(lang).unwrap();
            let theme = &ts.themes["base16-ocean.light"];

            let html = highlighted_html_for_string(code, &ss, syntax, theme).unwrap();
            let formatted = html
                .replace("\"", "\\\"")
                .lines()
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>()
                .join("<br>");

            let mut output = String::new();
            output.push('[');
            write!(output, r#"{{"name": "raw", "data": "<code {style}>"}},"#).expect("");
            write!(output, r#"{{"name": "raw", "data": "{formatted}"}},"#).expect("");
            output.push_str(r#"{"name": "raw", "data": "</code>"}"#);
            output.push(']');

            print!("{output}");
        }
        other => {
            eprintln!("Cannot convert code to {other}");
            return;
        }
    }
}
