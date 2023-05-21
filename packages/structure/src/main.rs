use serde_json::{from_str, json, Value};
use std::io::Read;
use std::{env, io};
use structure::*;

fn transform_label_to_key(input: Value) {
    let label = input["data"].as_str().unwrap();
    let structure = get_structure_list();

    if label.is_empty() {
        eprintln!("No label provided!");
        return;
    }

    for entry in structure {
        if Some(label) == entry["alias"].as_str() {
            if let Some(key) = entry["key"].as_str() {
                println!("[{}]", json!(key));
                return;
            }
        }
    }

    println!("[{}]", json!(label));
}

fn transform_element_number(input: Value) {
    let key = input["data"].as_str().unwrap().to_string();
    let structure = get_structure_list();
    let mut counter = StructureCounter::new();

    for entry in structure {
        let entry_key = entry["key"].as_str().unwrap_or("");
        let level = entry["level"].as_u64().unwrap_or(0) as usize;
        let entry_alias = entry["alias"].as_str().unwrap_or("");
        let element = entry["element"].as_str().unwrap_or("");

        match element {
            "numbered-heading" => counter.push_heading(level),
            "figure" => counter.push_figure(),
            "table" => counter.push_table(),
            _ => {}
        }

        if key == entry_key || key == entry_alias {
            let output = match element {
                "numbered-heading" => counter.get_heading(),
                "figure" => counter.get_figure(),
                "table" => counter.get_figure(),
                _ => key,
            };
            println!("[{}]", json!(output));
            return;
        }
    }

    eprintln!("Cannot find element with key={key}");
}

fn transform_heading(input: Value, to: &str, element: &str) {
    match to {
        "html" => {
            let mut json = vec![];
            let contents = input["data"].as_str().unwrap();
            let level_arg = input["arguments"]["level"].as_str().unwrap();
            let key = rand::random::<u64>();
            let level = level_arg.parse::<usize>().unwrap().clamp(1, 6);

            let structure_data = json!({
                "element": element,
                "level": level,
                "key": format!("{key}"),
                "contents": inline_content!(contents),
            })
            .to_string();

            json.push(json!(format!("<h{level}>")));

            if element == "numbered-heading" {
                let invocation = format!("[element-number]({key}) ");
                json.push(inline_content!(invocation));
            }

            json.push(inline_content!(contents));
            json.push(json!(format!("</h{level}>")));
            json.push(json!(
                {
                    "name": "list-push",
                    "arguments": {"name": "structure"},
                    "data": structure_data,
                }
            ));

            print!("{}", serde_json::to_string(&json).unwrap());
        }
        other => eprintln!("Cannot convert {element} to {other}!"),
    }
}

fn transform_standalone_heading(input: Value, to: &str) {
    match to {
        "html" => {
            let mut json = vec![];
            let contents = input["data"].as_str().unwrap();
            let level_arg = input["arguments"]["level"].as_str().unwrap();
            let level = level_arg.parse::<usize>().unwrap().clamp(1, 6);

            json.push(json!(format!("<h{level}>")));
            json.push(inline_content!(contents));
            json.push(json!(format!("</h{level}>")));

            print!("{}", serde_json::to_string(&json).unwrap());
        }
        other => eprintln!("Cannot convert standalone-heading to {other}!"),
    }
}

fn transform_toc(input: Value, to: &str) {
    let max_level = input["arguments"]["max-level"].as_u64().unwrap() as usize;
    let toc = TOC::build_from_list(max_level);

    match to {
        "html" => print!("{}", toc.to_html()),
        other => eprintln!("Cannot convert table-of-contents to {other}!"),
    }
}

fn transform(from: &str, to: &str) {
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        from_str(&buffer).unwrap()
    };

    match from {
        "table-of-contents" => transform_toc(input, to),
        "unnumbered-heading" => transform_heading(input, to, from),
        "numbered-heading" => transform_heading(input, to, from),
        "standalone-heading" => transform_standalone_heading(input, to),
        "element-number" => transform_element_number(input),
        "label-to-key" => transform_label_to_key(input),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn manifest() {
    print!(
        "{}",
        json!({
            "name": "structure",
            "version": "0.1",
            "description": "This package manages document structure to provide numbering for headings, figures and tables.",
            "transforms": [
                {
                    "from": "table-of-contents",
                    "to": ["any"],
                    "description": "Creates a table of contents using headings the document.",
                    "arguments": [
                        {"name": "max-level", "type": "uint", "default": 4, "description": "Specifies the highest level of headings that will be included in the TOC. Examples: 2 -> 1.1, 4 -> 1.1.1.1."},
                    ],
                    "variables": {
                        "structure": {"type": "list", "access": "read"}
                    }
                },
                {
                    "from": "unnumbered-heading",
                    "to": ["html"],
                    "description": "A heading that does not include a number and is not numbered in a table of contents.",
                    "type": "inline-module",
                    "arguments": [
                        {
                            "name": "level",
                            "description": "The level of the heading",
                            "default": "1"
                        }
                    ],
                    "variables": {
                        "structure": {"type": "list", "access": "push"}
                    }
                },
                {
                    "from": "numbered-heading",
                    "to": ["html"],
                    "type": "inline-module",
                    "description": "A heading that includes number and is numbered in a table of contents.",
                    "arguments": [
                        {
                            "name": "level",
                            "description": "The level of the heading",
                            "default": "1"
                        }
                    ],
                    "variables": {
                        "structure": {"type": "list", "access": "push"}
                    }
                },
                {
                    "from": "standalone-heading",
                    "to": ["html"],
                    "type": "inline-module",
                    "description": "A heading is not included in the document's structure or table of contents.",
                    "arguments": [
                        {
                            "name": "level",
                            "description": "The level of the heading",
                            "default": "1"
                        }
                    ],
                },
                {
                    "from": "element-number",
                    "to": ["any"],
                    "arguments": [],
                    "variables": {
                        "structure": {"type": "list", "access": "read"}
                    }
                },
                {
                    "from": "label-to-key",
                    "to": ["any"],
                    "arguments": [],
                    "variables": {
                        "structure": {"type": "list", "access": "read"}
                    }
                },
            ],

        })
    );
}

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
