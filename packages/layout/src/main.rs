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
        json!(
            {
            "name": "layout",
            "version": "0.1",
            "description": "This package provides primitive layout modules.",
            "transforms": [
                {
                    "from": "newline",
                    "to": ["html", "latex"],
                    "arguments": [],
                },
                {
                    "from": "newpage",
                    "to": ["html", "latex"],
                    "arguments": [],
                },
                {
                    "from": "row",
                    "to": ["html"],
                    "arguments": [
                        {"name": "separator", "default": ",", "description": "The pattern used to separate items in the input content." },
                        {"name": "gap", "default": "10", "description":
                            "The gap between items. You can \
                            optionally add a css unit, otherwise \
                            rem will be used."
                        },
                        {"name": "max_width", "default": "none", "description":
                            "Max width of the row. You can optionally add \
                            a css unit, otherwise rem will be used. \
                            Note that content that is too wide will \
                            be cropped if used without wrapping."
                        },
                        {"name": "wrap", "default": "false", "type": ["true", "false"], "description": "Decides if items will wrap around to new rows."},
                    ]
                },
                {
                    "from": "center",
                    "to": ["html"],
                    "arguments": [
                        {"name": "separator", "default": ",", "description": "The pattern used to separate items in the input content." },
                        {"name": "gap", "default": "10", "description":
                            "The gap between items. You can \
                            optionally add a css unit, otherwise \
                            rem will be used."
                        },
                        {"name": "max_width", "default": "none", "description":
                            "Max width of the row. You can optionally add \
                            a css unit, otherwise rem will be used. \
                            Note that content that is too wide will \
                            be cropped if used without wrapping."
                        },
                        {"name": "wrap", "default": "false", "type": ["true", "false"], "description": "Decides if items will wrap around to new rows."},
                    ]
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
        "row" | "center" => transform_flex(from, to, input),
        "newline" => transform_newline(to, input),
        "newpage" => transform_newpage(to, input),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_newline(to: &str, input: Value) {
    if let Value::String(data) = &input["data"] {
        if !data.is_empty() {
            eprintln!("The newline module was fed with some input. Maybe this was a mistake? Consider adding delimiters like this: [newline]().");
        }
    }

    match to {
        "latex" => println!("[{}]", json!({"name": "raw", "data": "\\\\"})),
        "html" => println!("[{}]", json!({"name": "raw", "data": "<br/>"})),
        other => eprintln!("Cannot convert to '{other}' format."),
    }
}

fn transform_newpage(to: &str, input: Value) {
    if let Value::String(data) = &input["data"] {
        if !data.is_empty() {
            eprintln!("The newpage module was fed with some input. Maybe this was a mistake? Consider adding delimiters like this: [newpage]().");
        }
    }

    match to {
        "latex" => println!("[{}]", json!({"name": "raw", "data": "\\newpage"})),
        "html" => println!(
            "[{}]",
            json!({"name": "raw", "data": r#"<div style="break-after: page;"></div>"#})
        ),
        other => eprintln!("Cannot convert to '{other}' format."),
    }
}

fn transform_flex(from: &str, to: &str, input: Value) {
    macro_rules! get_arg {
        ($input:expr, $arg:expr) => {
            if let Value::String(val) = &$input["arguments"][$arg] {
                val
            } else {
                panic!("No {} argument was provided", $arg);
            }
        };
    }

    match to {
        "html" => {
            let content = input["data"].as_str().unwrap();
            let separator = get_arg!(input, "separator");
            let gap = get_arg!(input, "gap");
            let max_width = get_arg!(input, "max_width");
            let wrap = get_arg!(input, "wrap");

            let style = match from {
                "row" => get_style("row", gap, max_width, wrap),
                "center" => get_style("center", gap, max_width, wrap),
                other => panic!("Unexpected transform from {other}"),
            };

            let open = json!({"name": "raw", "data": format!("<div {style}>")});
            let items = content
                .split(separator)
                .map(|item| json!({"name": "block_content", "data": item}).to_string())
                .collect::<Vec<String>>()
                .join(",");
            let close = json!({"name": "raw", "data": format!("</div>")});

            print!("[{open},{items},{close}]");
        }
        other => {
            eprintln!("Cannot convert {from} to {other}");
        }
    }
}

fn get_style(layout: &str, gap: &str, max_width: &str, wrap: &str) -> String {
    let mut style = match layout {
        "center" => String::from(
            "style=\"display:flex; \
                justify-content: center; \
                margin-left: auto; \
                margin-right: auto; ",
        ),
        "row" => String::from("style=\"display:flex; "),
        _ => panic!("Unexpected layout: {layout}"),
    };

    let units = vec![
        "cm", "mm", "in", "px", "pt", "pc", "em", "ex", "ch", "rem", "vw", "vh", "vmin", "vmax",
        "%",
    ];

    let gap = gap.replace(" ", "");
    let num = gap
        .chars()
        .take_while(|c| c.is_numeric())
        .collect::<String>();
    let unit = gap.chars().skip(num.len()).collect::<String>();
    if num.parse::<usize>().is_ok() {
        if !unit.is_empty() {
            if units.iter().any(|s| *s == unit.as_str()) {
                write!(style, "gap: {num}{unit}; ").unwrap();
            } else {
                write!(style, "gap: {num}rem; ").unwrap();
                eprintln!("Unexpected value for argument: gap - invalid unit")
            }
        } else {
            write!(style, "gap: {num}rem; ").unwrap();
        }
    } else {
        eprintln!("Unexpected value for argument: gap - expected a number")
    }

    let max_width = max_width.replace(" ", "");
    let num = max_width
        .chars()
        .take_while(|c| c.is_numeric())
        .collect::<String>();
    let unit = max_width.chars().skip(num.len()).collect::<String>();
    if num.parse::<usize>().is_ok() {
        if !unit.is_empty() {
            if units.iter().any(|s| *s == unit.as_str()) {
                write!(style, "max-width: {num}{unit}; ").unwrap();
            } else {
                write!(style, "max-width: {num}rem; ").unwrap();
                eprintln!("Unexpected value for argument: max_width - invalid unit")
            }
        } else {
            write!(style, "max-width: {num}rem; ").unwrap();
        }
    } else {
        write!(style, "max-width: 100%; ").unwrap();
        if max_width != "none" {
            eprintln!("Unexpected value for argument: max_width - expected a number")
        }
    }

    if wrap == "true" {
        write!(style, "flex-wrap: wrap; ").unwrap();
    } else {
        write!(style, "overflow: hidden; ").unwrap();
        if wrap != "false" {
            eprintln!("Unexpected value for argument: wrap - expected true/false")
        }
    }

    style.push('\"');
    style
}
