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
                    "unknown-content": true
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
                            differs-from: compile the content if the variable differs from the given value. \
                            defined: compile the content if the variable is defined. \
                            undefined: compile the content if the variable is undefined.
                            ",
                            "default": "defined",
                            "type": ["equals", "differs-from", "defined", "undefined"]
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
                    },
                    "unknown-content": true
                },
                {
                    "from": "if-set",
                    "to": ["any"],
                    "description": "Conditionally compile content based on a set. Example: [if-set imports contains-all foo,bar,baz]",
                    "arguments": [
                        {
                            "name": "set",
                            "description":
                                "The set to use for conditional compilation"
                        },
                        {
                            "name": "check",
                            "description": "Specifies the check to do to the value. \
                            does-not-contain: compile the content if the set does not contain the given string. \
                            contains: compile the content if the set contains the given string. \
                            contains-all: compile the content if the set contains all of the comma-separated strings. \
                            contains-any: compile the content if the set contains any of the comma-separated strings. \
                            contains-none: compile the content if the set contains none of the comma-separated strings. \
                            non-empty: compile the content if the set is defined and has at least one string. \
                            empty-or-undefined: compile the content if the set is undefined or is empty
                            ",
                            "default": "non-empty",
                            "type": ["does-not-contain", "contains", "contains-all", "contains-any", "contains-none", "non-empty", "empty-or-undefined"]
                        },
                        {
                            "name": "value",
                            "description": "Specifies the value to compare to. If the operation allows for multiple values, they should be comma-separated",
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
                        "$set": {"type": "set", "access": "read"}
                    },
                    "unknown-content": true
                },
                {
                    "from": "if-list",
                    "to": ["any"],
                    "description": "Conditionally compile content based on a list. Example: [if-list authors contains Jonathan]",
                    "arguments": [
                        {
                            "name": "list",
                            "description":
                                "The list to use for conditional compilation"
                        },
                        {
                            "name": "check",
                            "description": "Specifies the check to do to the value. \
                            does-not-contain: compile the content if the list does not contain the given string. \
                            contains: compile the content if the list contains the given string. \
                            contains-all: compile the content if the list contains all of the comma-separated strings. \
                            contains-any: compile the content if the list contains any of the comma-separated strings. \
                            contains-none: compile the content if the list contains none of the comma-separated strings. \
                            non-empty: compile the content if the list is defined and has at least one string. \
                            empty-or-undefined: compile the content if the list is undefined or is empty
                            ",
                            "default": "non-empty",
                            "type": ["does-not-contain", "contains", "contains-all", "contains-any", "contains-none", "non-empty", "empty-or-undefined"]
                        },
                        {
                            "name": "value",
                            "description": "Specifies the value to compare to. If the operation allows for multiple values, they should be comma-separated",
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
                        "$list": {"type": "list", "access": "read"}
                    },
                    "unknown-content": true
                }
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
        "if-set" => transform_if_collection(input, true),
        "if-list" => transform_if_collection(input, false),
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
            var_val
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

fn transform_if_collection(input: Value, is_set: bool) {
    let key = input["arguments"][if is_set { "set" } else { "list" }]
        .as_str()
        .unwrap();
    let mut env_values: Vec<String> =
        serde_json::from_str(&env::var(key).unwrap_or("[]".to_string())).unwrap();
    let case_sensitive = input["arguments"]["case-sensitive"].as_str() == Some("case-sensitive");
    if !case_sensitive {
        env_values.iter_mut().for_each(|x| *x = x.to_lowercase())
    }

    let check = input["arguments"]["check"].as_str().unwrap();
    let value = {
        let arg = input["arguments"]["value"].as_str().unwrap();
        if case_sensitive {
            arg.to_string()
        } else {
            arg.to_lowercase()
        }
    };

    // We have a Vec<String> and want to check if it contains a &str, but there is no way of doing
    // that using .contains since it expects a &String (and you can make a &String to a &str but
    // not the other way around). Because of that, we do .iter().any(|v| v == x) with this macro
    macro_rules! contains {
        ($c:expr, $str:expr) => {
            $c.iter().any(|v| v == $str)
        };
    }

    let compile = if check == "non-empty" || check == "empty-or-undefined" {
        env_values.is_empty() == (check == "empty-or-undefined")
    } else if check == "contains" || check == "does-not-contain" {
        // cannot use .contains here due to coercion rules
        contains!(env_values, &value) == (check == "contains")
    } else {
        let split_values: Vec<&str> = value.split_terminator(',').collect();

        match check {
            "contains-all" => split_values.into_iter().all(|v| contains!(env_values, v)),
            "contains-any" => split_values.into_iter().any(|v| contains!(env_values, v)),
            "contains-none" => split_values.into_iter().all(|v| !contains!(env_values, v)),
            _ => unreachable!("Invalid check {check}"),
        }
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
