use serde_json::{from_str, json, Value};
use std::env;
use TocEntryType::*;

#[macro_export]
macro_rules! inline_content {
    ($expr:expr) => {
        json!({
            "name": "inline_content",
            "data": $expr
        })
    }
}

pub struct StructureCounter {
    headings: Vec<u64>,
    figures: u64,
    tables: u64,
}

#[derive(PartialEq)]
enum TocEntryType {
    Numbered,
    Unnumbered,
    Empty,
}

struct TocEntry {
    id: Option<String>,
    contents: Option<Value>,
    children: Vec<TocEntry>,
    mode: TocEntryType,
}

pub struct Toc {
    table: TocEntry,
    max_level: usize,
}

impl StructureCounter {
    pub fn new() -> Self {
        Self {
            headings: vec![0],
            figures: 0,
            tables: 0,
        }
    }
    // Enforce a limit of 256 levels, to avoid creating an extremely long vec from malformed input.
    pub fn push_heading(&mut self, level: usize) {
        if level < 1 || level > u8::MAX as usize {
            return;
        }

        if level == 1 {
            self.figures = 0;
            self.tables = 0;
        }

        while self.headings.len() < level {
            self.headings.push(0);
        }

        self.headings[level - 1] += 1;

        for counter in &mut self.headings[level..] {
            *counter = 0;
        }
    }

    pub fn push_figure(&mut self) {
        self.figures += 1;
    }

    pub fn push_table(&mut self) {
        self.tables += 1;
    }

    pub fn get_heading(&self) -> String {
        self.headings
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<String>>()
            .join(".")
            .trim_end_matches(".0")
            .to_string()
    }

    pub fn get_figure(&self) -> String {
        format!("{}.{}", self.headings[0], self.figures)
    }

    pub fn get_table(&self) -> String {
        format!("{}.{}", self.headings[0], self.tables)
    }
}

impl TocEntry {
    fn numbered(id: String, contents: Value) -> Self {
        TocEntry {
            id: Some(id),
            contents: Some(contents),
            children: vec![],
            mode: Numbered,
        }
    }

    fn unnumbered(id: String, contents: Value) -> Self {
        TocEntry {
            id: Some(id),
            contents: Some(contents),
            children: vec![],
            mode: Unnumbered,
        }
    }

    fn empty() -> Self {
        TocEntry {
            id: None,
            contents: None,
            children: vec![],
            mode: Empty,
        }
    }
}

impl Toc {
    // Build and return a TOC from the elements in "structure". Currently, only "numbered-heading"
    // and "unnumbered-heading" are added.
    pub fn build_from_list(max_level: usize) -> Self {
        let structure = get_structure_list();
        let mut toc = Self {
            table: TocEntry::empty(),
            max_level,
        };

        for entry in structure {
            let element = entry["element"].as_str().unwrap();
            let id = entry["key"].as_str().unwrap_or("").to_string();
            let level = entry["level"].as_u64().unwrap_or(0) as usize;
            let contents = entry["contents"].clone();

            match element {
                "numbered-heading" => toc.push(level, TocEntry::numbered(id, contents)),
                "unnumbered-heading" => toc.push(level, TocEntry::unnumbered(id, contents)),
                _ => {}
            }
        }

        toc
    }

    // Push an element to the TOC. It works a bit like tree traversal. If there is a jump in level
    // (e.g. "# heading" followed by "### heading"), empty entries will be created to accommodate.
    // If entries are numbered, the example would result in "1 heading" followed by "1.0.1 heading".
    fn push(&mut self, level: usize, entry: TocEntry) {
        if level < 1 || level > self.max_level {
            return;
        }

        let mut pointer = &mut self.table;
        for _ in 1..level {
            let last = pointer.children.pop().unwrap_or(TocEntry::empty());
            pointer.children.push(last);
            pointer = pointer.children.last_mut().unwrap();
        }

        pointer.children.push(entry);
    }

    pub fn to_html(&self) -> String {
        let mut counter = StructureCounter::new();
        let mut json = vec![];
        to_html_helper(&self.table, &mut counter, &mut json, 0);
        serde_json::to_string(&json).unwrap()
    }
}

//
fn to_html_helper(
    pointer: &TocEntry,
    counter: &mut StructureCounter,
    json: &mut Vec<Value>,
    level: usize,
) {
    if pointer.mode == Numbered {
        counter.push_heading(level);

        let number = counter.get_heading();
        let id = pointer.id.as_ref().unwrap().clone();
        let contents = pointer.contents.as_ref().unwrap().clone();

        json.push(json!(format!("<li><a href=\"#{id}\">{number} ")));
        json.push(contents);
        json.push(json!("</a></li>"));
    } else if pointer.mode == Unnumbered {
        let id = pointer.id.as_ref().unwrap().clone();
        let contents = pointer.contents.as_ref().unwrap().clone();

        json.push(json!(format!("<li><a href=\"#{id}\">")));
        json.push(contents);
        json.push(json!("</a></li>"));
    }

    if !pointer.children.is_empty() {
        json.push(json!("<ul style=\"list-style-type: none\">"));
        for child in &pointer.children {
            to_html_helper(child, counter, json, level + 1);
        }
        json.push(json!("</ul>"));
    }
}

// Get a list of Values from the "structure" environment variable
// Labels are not added to the list. Instead the are attached to the
// previous element (as an alias) if possible.
pub fn get_structure_list() -> Vec<Value> {
    let mut structure: Vec<Value> = vec![];
    let strings = {
        let var = env::var("structure").unwrap_or("[]".to_string());
        from_str::<Vec<String>>(&var).unwrap()
    };

    for s in &strings {
        let entry = match from_str::<Value>(s) {
            Ok(entry) => entry,
            _ => continue,
        };

        match (entry["element"].as_str(), structure.last_mut()) {
            (Some("label"), Some(last)) => last["alias"] = entry["key"].clone(),
            (Some(_), _) => structure.push(entry),
            (None, _) => {}
        }
    }

    structure
}
