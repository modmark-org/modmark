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
    print!("{}", serde_json::to_string(&json!(
        {
        "name": "layout",
        "version": "0.1",
        "description": "This package provides primitive layout modules.",
        "transforms": [
            {
                "from": "row",
                "to": ["html"],
                "arguments": [
                    {"name": "separator", "default": ",", "description": "The pattern used to separate items in the input content." },
                    {"name": "gap", "default": "10", "description": "The gap between items given in pixels." },
                    {"name": "max_width", "default": "none", "description":
                        "Max width of the row given in pixels. \
                        Note that content that is too wide will \
                        be cropped if used without wrapping."
                    },
                    {"name": "wrap", "default": "false", "description": "true/false - Decides if items will wrap around to new rows."},
                ]
            },
            {
                "from": "center",
                "to": ["html"],
                "arguments": [
                    {"name": "separator", "default": ",", "description": "The pattern used to separate items in the input content." },
                    {"name": "gap", "default": "10", "description": "The gap between items given in pixels." },
                    {"name": "max_width", "default": "none", "description": "Max width of the row given in pixels"},
                    {"name": "wrap", "default": "false", "description": "true/false - Decides if items will wrap around to new rows."},
                ]
            }
        ]
        }
    ))
        .unwrap());
}

fn transform(from: &str, to: &str) {
    match from {
        "row" | "center" => transform_flex(from, to),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_flex(from: &str, to: &str) {
    macro_rules! get_arg {
        ($input:expr, $arg:expr) => {
            if let Value::String(val) = &$input["arguments"][$arg] {
                val
            } else {
                panic!("No {} argument was provided", $arg);
            }
        }
    }
    match to {
        "html" => {

            let input: Value = {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();
                serde_json::from_str(&buffer).unwrap()
            };

            let content = input["data"].as_str().unwrap();
            let separator = get_arg!(input, "separator");
            let gap = get_arg!(input, "gap");
            let max_width = get_arg!(input, "max_width");
            let wrap = get_arg!(input, "wrap");

            let style = if from == "row" {
                get_row_style(gap, max_width, wrap)
            } else if from == "center" {
                get_center_style(gap, max_width, wrap)
            } else {
                panic!("Expected center or row module")
            };

            let mut output = String::new();
            let open = format!("<div {style}>");
            write!(output, "{},", json!({"name": "raw", "data": open})).unwrap();
            for item in content.split(separator) {
                write!(output, "{},", json!({"name": "block_content", "data": item})).unwrap();
            }
            write!(output, "{}", json!({"name": "raw", "data": "</div>"})).unwrap();
            print!("[{output}]");
        }
        other => {
            eprintln!("Cannot convert {from} to {other}");
        }
    }
}

fn get_row_style(gap: &str, max_width: &str, wrap: &str) -> String {
    let mut style = String::from("style=\"display:flex; ");

    if gap.parse::<usize>().is_ok() {
        write!(style, "gap: {gap}px; ").unwrap();
    }

    if max_width.parse::<usize>().is_ok() {
        write!(style, "max-width: {max_width}px; ").unwrap();
    } else {
        write!(style, "max-width: 100%; ").unwrap();
    }

    if wrap == "true" {
        write!(style, "flex-wrap: wrap; ").unwrap();
    } else {
        write!(style, "overflow: hidden; ").unwrap();
    }

    style.push('\"');
    style
}

fn get_center_style(gap: &str, max_width: &str, wrap: &str) -> String {
    let mut style = String::from(
        "style=\"display:flex; \
        justify-content: center; \
        margin-left: auto; \
        margin-right: auto; "
    );

    if gap.parse::<usize>().is_ok() {
        write!(style, "gap: {gap}px; ").unwrap();
    }

    if max_width.parse::<usize>().is_ok() {
        write!(style, "max-width: {max_width}px; ").unwrap();
    } else {
        write!(style, "max-width: 100%; ").unwrap();
    }

    if wrap == "true" {
        write!(style, "flex-wrap: wrap; ").unwrap();
    } else {
        write!(style, "overflow: hidden; ").unwrap();
    }

    style.push('\"');
    style
}