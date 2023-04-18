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
                {
                    "from": "if-const",
                    "to": ["any"],
                    "description": "Conditionally compile content based on a constant. Example: [if-const theme equals dark]",
                    "arguments": [
                        {
                            "name": "constant",
                            "description":
                                "The constant to use for conditional compilation"
                        },
                        {
                            "name": "check",
                            "description": "Specifies the check to do to the value. \
                            equals: compile the content if the variable equals the given value. \
                            differs-to: compile the content if the variable differs from the given value. \
                            defined: compile the content if the variable is defined. \
                            undefined: compile the content if the variable is undefined.
                            ",
                            "default": "defined",
                            "type": ["equals", "differs-to", "defined", "undefined"]
                        },
                        {
                            "name": "value",
                            "description": "Specifies the value to compare to",
                            "default": ""
                        },
                        {
                            "name": "case-sensitive",
                            "description": "Specifies if 'equals/differs' checks are case sensitive.",
                            "default": "case-sensitive",
                            "type": ["case-sensitive", "case-insensitive"]
                        }
                    ],
                    "variables": {
                        "$constant": {"type": "constant", "access": "read"}
                    }
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
        "if-const" => transform_if_const(input),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_if_const(input: Value) {
    let var_name = input["arguments"]["constant"].as_str().unwrap();
    let check = input["arguments"]["check"].as_str().unwrap();
    let compile = if check == "defined" || check == "undefined" {
        (check == "defined") == (env::var(var_name).is_ok())
    } else if let Ok(var_val) = env::var(var_name) {
        let case_sensitive = input["arguments"]["case-sensitive"].as_str().unwrap() == "true";
        let var_val = if case_sensitive {
            var_val.to_string()
        } else {
            var_val.to_lowercase()
        };
        let value = input["arguments"]["value"].as_str().unwrap();

        let cmp_val = if case_sensitive {
            value.to_string()
        } else {
            value.to_lowercase()
        };

        (cmp_val == var_val) == (check == "equals")
    } else {
        false
    };

    if compile {
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
