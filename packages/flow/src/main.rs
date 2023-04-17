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
        json!(
            {
            "name": "flow",
            "version": "0.1",
            "description": "This package provides modules to control flow during compilation.",
            "transforms": [
                {
                    "from": "if",
                    "to": ["any"],
                    "description": "Conditionally compile content based on output format.",
                    "arguments": [
                        {
                            "name": "format",
                            "description":
                                "\
                                Specifies the output format, which the conditional compilation \
                                depends on. You can prefix the format with an exclamation mark \
                                (such as format=!html) to invert the outcome.\
                                "
                        },
                    ],
                },
            ]
            }
        )
    );
}

fn transform(from: &str, to: &str) {
    let input = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };

    match from {
        "if" => transform_if(to, input),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_if(output_format: &str, input: Value) {
    let cmp_format = input["arguments"]["format"]
        .as_str()
        .unwrap()
        .to_lowercase();
    let result = match cmp_format.strip_prefix('!') {
        Some(cmp_format) => cmp_format != output_format,
        None => cmp_format == output_format,
    };

    if result {
        let inline = input["inline"].as_bool().unwrap();
        let body = input["data"].as_str().unwrap();
        let json = if inline {
            json!({"name": "inline_content", "data": body}).to_string()
        } else {
            json!({"name": "block_content", "data": body}).to_string()
        };
        print!("[{json}]")
    } else {
        print!("[]")
    }
}
