use serde_json::{from_str, json, Value};
use std::io::Read;
use std::{env, io};

enum TOCEntryMode {
    Numbered,
    Unnumbered,
    Empty,
}

struct TOCEntry {
    id: Option<String>,
    contents: Option<String>,
    children: Vec<TOCEntry>,
    mode: TOCEntryMode,
}

impl TOCEntry {
    fn numbered(id: String, contents: String) -> Self {
        TOCEntry {
            id: Some(id),
            contents: Some(contents),
            children: vec![],
            mode: TOCEntryMode::Numbered,
        }
    }

    fn unnumbered(id: String, contents: String) -> Self {
        TOCEntry {
            id: Some(id),
            contents: Some(contents),
            children: vec![],
            mode: TOCEntryMode::Unnumbered,
        }
    }

    fn empty() -> Self {
        TOCEntry {
            id: None,
            contents: None,
            children: vec![],
            mode: TOCEntryMode::Empty,
        }
    }
}

struct TOC {
    table: TOCEntry,
    max_level: usize,
}

impl TOC {
    fn new(max_level: usize) -> Self {
        Self {
            table: TOCEntry::empty(),
            max_level,
        }
    }

    fn build_from_list(max_level: usize, structure: Vec<String>) -> Self {
        let mut toc = Self::new(max_level);
        for i in 0..structure.len() {
            let entry = match from_str::<Value>(&structure[i]) {
                Ok(entry) => entry,
                _ => continue,
            };

            let element = entry["element"].as_str().unwrap_or("").to_string();
            let id = entry["key"].as_str().unwrap_or("").to_string();
            let contents = entry["contents"].as_str().unwrap_or("").to_string();

            if element == "numbered-heading" {
                if let Some(level) = entry["level"].as_u64() {
                    let entry = TOCEntry::numbered(id, contents);
                    toc.push(level as usize, entry);
                }
            } else if element == "unnumbered-heading" {
                if let Some(level) = entry["level"].as_u64() {
                    let entry = TOCEntry::unnumbered(id, contents);
                    toc.push(level as usize, entry);
                }
            }
        }
        toc
    }

    fn push(&mut self, level: usize, entry: TOCEntry) {
        if level > self.max_level {
            return;
        }

        let mut pointer = &mut self.table;
        let mut children_level = 1;

        while children_level < level {
            let last = pointer.children.pop().unwrap_or(TOCEntry::empty());
            pointer.children.push(last);
            pointer = pointer.children.last_mut().unwrap();
            children_level += 1;
        }

        pointer.children.push(entry);
    }

    fn to_html(&self) -> String {
        let pointer = &self.table;
        let mut counters = vec![0; self.max_level as usize];
        let mut json = vec![];
        self.to_html_helper(pointer, &mut json, &mut counters, 1);
        serde_json::to_string(&json).unwrap_or(String::from("[]"))
    }

    fn to_html_helper(
        &self,
        pointer: &TOCEntry,
        json: &mut Vec<Value>,
        counters: &mut Vec<usize>,
        level: usize,
    ) {
        if !pointer.children.is_empty() {
            json.push(json!("<ul style=\"list-style-type: none\">"));
        }

        for child in &pointer.children {
            if let TOCEntryMode::Empty = child.mode {
                self.to_html_helper(child, json, counters, level + 1);
                continue;
            }

            if let Some(counter) = counters.get_mut(level - 1) {
                if let TOCEntryMode::Numbered = child.mode {
                    *counter += 1;
                }
            }

            for counter in &mut counters[level..] {
                *counter = 0;
            }

            if let Some(contents) = child.contents.as_ref() {
                json.push(json!("<li>"));

                if let Some(id) = &child.id {
                    json.push(json!(format!("<a href=\"#{id}\">")));
                }

                if let TOCEntryMode::Numbered = child.mode {
                    let numbering = &counters[..level]
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<String>>()
                        .join(".");
                    json.push(json!(format!("{numbering} ")));
                }

                if let Ok(v) = from_str(contents) {
                    json.push(v);
                }

                if let Some(_id) = &child.id {
                    json.push(json!("</a>"));
                }

                json.push(json!("</li>"));
            }

            self.to_html_helper(child, json, counters, level + 1);
        }

        if !pointer.children.is_empty() {
            json.push(json!("</ul>"));
        }
    }
}

// Attach labels to the previous element if possible
fn attach_labels(structure: &mut Vec<String>) {
    for i in (1..structure.len()).rev() {
        let mut prev: Value = match from_str(&structure[i - 1]) {
            Ok(v) => v,
            _ => continue,
        };

        let entry: Value = match from_str(&structure[i]) {
            Ok(v) => v,
            _ => continue,
        };

        if entry["element"].as_str() != Some("label") {
            continue;
        }

        if prev["element"].as_str() == Some("label") {
            continue;
        }

        if entry["key"] == Value::Null {
            continue;
        }

        prev["alias"] = entry["key"].clone();

        structure[i - 1] = prev.to_string();
    }
}

fn transform_label_to_id(input: Value) {
    let label = input["data"].as_str().unwrap_or("");
    let mut structure: Vec<String> = {
        let var = env::var("structure").unwrap_or("[]".to_string());
        from_str(&var).unwrap()
    };

    attach_labels(&mut structure);

    for i in 0..structure.len() {
        let entry = match from_str::<Value>(&structure[i]) {
            Ok(v) => v,
            _ => continue,
        };

        if Some(label) == entry["alias"].as_str() {
            if let Some(id) = entry["key"].as_str() {
                println!("[{}]", json!(id));
                return;
            }
        }
    }
    if label.is_empty() {
        eprintln!("No label provided!");
    } else {
        println!("[{}]", json!(label));
    }
}

fn transform_element_number(input: Value) {
    let mut headings = vec![0; 16]; // hmm
    let mut figures = 0;
    let mut tables = 0;

    let input_id = match input["data"].as_str() {
        Some(data) => data,
        None => {
            eprintln!("No label provided!");
            return;
        }
    };

    let mut structure: Vec<String> = {
        let var = env::var("structure").unwrap_or("[]".to_string());
        from_str(&var).unwrap()
    };

    attach_labels(&mut structure);

    for i in 0..structure.len() {
        let entry = match from_str::<Value>(&structure[i]) {
            Ok(json) => json,
            _ => continue,
        };

        let id = entry["key"].as_str().unwrap_or("");
        let alias = entry["alias"].as_str().unwrap_or("");
        let element = entry["element"].as_str().unwrap_or("");

        if element == "figure" {
            figures += 1;
        } else if element == "table" {
            tables += 1;
        } else if element == "numbered-heading" {
            if let Some(level) = entry["level"].as_u64().map(|l| l as usize) {
                if 0 < level && level < headings.len() {
                    headings[level - 1] += 1;
                    for counter in &mut headings[level..] {
                        *counter = 0;
                    }
                }
            }
        }

        if input_id != id && input_id != alias {
            continue;
        }

        if element == "figure" {
            println!("[{}]", json!(format!("{}.{}", headings[0], figures)));
            return;
        } else if element == "table" {
            println!("[{}]", json!(format!("{}.{}", headings[0], tables)));
            return;
        } else if element == "numbered-heading" {
            println!(
                "[{}]",
                json!(headings
                    .iter()
                    .map(|counter| counter.to_string())
                    .collect::<Vec<String>>()
                    .join(".")
                    .trim_end_matches(".0"))
            );
            return;
        }
    }

    eprintln!("Cannot find element with id={input_id}");
    println!("[{}]", json!(input_id));
}

fn transform_heading(input: Value, to: &str, numbered: bool) {
    match to {
        "html" => {
            let mut json = vec![];
            let contents = input["data"].as_str().unwrap();
            let level_arg = input["arguments"]["level"].as_str().unwrap();
            let id = rand::random::<u64>();
            let level = level_arg.parse::<usize>().unwrap().clamp(1, 6);

            let element = if numbered {
                format!("numbered-heading")
            } else {
                format!("unnumbered-heading")
            };

            // can probably avoided if other functions are improved
            let escaped_content = json!({
                "name": "inline_content",
                "data": contents
            })
            .to_string();

            let structure_data = json!({
                "element": element,
                "level": level,
                "key": format!("{id}"),
                "contents": escaped_content,
            })
            .to_string();

            json.push(json!(format!("<h{level}>")));

            if numbered {
                json.push(json!(
                    {
                        "name": "inline_content",
                        "data": format!("[element-number]({id}) ")
                    }
                ));
            }

            json.push(json!({"name": "inline_content", "data": contents}));
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
        other => eprintln!("Cannot convert heading to {other}!"),
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
            json.push(json!({"name": "inline_content", "data": contents}));
            json.push(json!(format!("</h{level}>")));

            print!("{}", serde_json::to_string(&json).unwrap());
        }
        other => eprintln!("Cannot convert heading to {other}!"),
    }
}

fn transform_toc(input: Value, to: &str) {
    let max_level = input["arguments"]["max-level"].as_u64().unwrap() as usize;
    let structure: Vec<String> = {
        let var = env::var("structure").unwrap_or("[]".to_string());
        from_str(&var).unwrap()
    };
    let toc = TOC::build_from_list(max_level, structure);

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
        "unnumbered-heading" => transform_heading(input, to, false),
        "numbered-heading" => transform_heading(input, to, true),
        "standalone-heading" => transform_standalone_heading(input, to),
        "element-number" => transform_element_number(input),
        "label-to-id" => transform_label_to_id(input),
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
                    "from": "label-to-id",
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
