use std::str::FromStr;

use serde_json::{json, Value};

macro_rules! module {
    ($name:expr, $data:expr $(,$($args:tt)*)?) => {json!({"name": $name $(,"arguments":$($args)*)*, "data": $data})}
}

macro_rules! import {
    ($package:expr) => {module!("set-add", $package, {"name": "imports"})}
}

macro_rules! inline_content {
    ($package:expr) => {
        module!("inline_content", $package, {})
    };
}

#[derive(Debug)]
pub struct InvalidListError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// Options in ordered types represent if they have a specified start value
enum OrderedType {
    Decimal,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

struct InvalidItem;

impl FromStr for ListType {
    type Err = InvalidItem;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ListType::*;
        use OrderedType::*;
        let s = s.trim().split_ascii_whitespace().next().unwrap_or("");
        let start_str = if s.starts_with('(') && s.ends_with(')') {
            &s[1..s.len() - 1]
        } else if s.ends_with(')') || s.ends_with('.') {
            &s[0..s.len() - 1]
        } else {
            ""
        };

        // Edge cases for i and I which should be roman and not alpha.
        if start_str == "i" {
            return Ok(OrderedList(1, LowerRoman));
        }
        if start_str == "I" {
            return Ok(OrderedList(1, UpperRoman));
        }

        if !start_str.is_empty() {
            if let Ok(start) = start_str.parse::<u32>() {
                return Ok(OrderedList(start, Decimal));
            } else if start_str.len() == 1
                && start_str.chars().next().unwrap().is_ascii_alphabetic()
            {
                let c = start_str.chars().next().unwrap();
                let order = alpha_to_start(c);
                if let Some(start) = order {
                    return if c.is_ascii_lowercase() {
                        Ok(OrderedList(start, LowerAlpha))
                    } else {
                        Ok(OrderedList(start, UpperAlpha))
                    };
                }
            } else if let Some(start) = roman::from(&start_str.to_ascii_uppercase()) {
                return if start_str.chars().next().unwrap().is_ascii_lowercase() {
                    Ok(OrderedList(start.unsigned_abs(), LowerRoman))
                } else {
                    Ok(OrderedList(start.unsigned_abs(), UpperRoman))
                };
            }
        }

        // If we have not already returned it is a bullet item or invalid.
        match s {
            "-" | "+" | "*" => Ok(UnorderedList),
            _ => Err(InvalidItem),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ListType {
    OrderedList(u32, OrderedType),
    UnorderedList,
}

#[derive(Debug)]
pub enum ListItem {
    Content(String),
    List(List),
}

impl ListItem {
    fn to_html_vec(&self) -> Vec<Value> {
        use ListItem::*;
        match self {
            Content(content) => {
                let mut json_vec = vec![];
                json_vec.push(Value::from("<li>"));
                json_vec.push(inline_content!(content));
                json_vec.push(Value::from("</li>"));
                json_vec
            }
            List(list) => list.to_html_vec(),
        }
    }

    fn to_latex_vec(&self) -> Vec<Value> {
        use ListItem::*;
        match self {
            Content(content) => {
                let mut json_vec = vec![];
                json_vec.push(Value::from(r"\item "));
                json_vec.push(inline_content!(content));
                json_vec.push(Value::from("\n"));
                json_vec
            }
            List(list) => list.to_latex_vec(),
        }
    }
}

#[derive(Debug)]
pub struct List {
    list_type: ListType,
    items: Vec<ListItem>,
}

impl List {
    fn max_depth(&self) -> usize {
        self.items
            .iter()
            .map(|item| match item {
                ListItem::Content(_) => 1,
                ListItem::List(sub_list) => sub_list.max_depth() + 1,
            })
            .max()
            .unwrap_or(1)
    }

    fn jumps_multiple_levels(&self) -> bool {
        if !self.items.is_empty() && matches!(self.items[0], ListItem::List(_)) {
            return true;
        }
        for item in &self.items {
            match item {
                ListItem::Content(_) => {}
                ListItem::List(sub_list) => {
                    if sub_list.jumps_multiple_levels() {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn opening_html_tag(&self) -> String {
        use ListType::*;
        use OrderedType::*;
        match self.list_type {
            UnorderedList => "<ul>".to_string(),
            OrderedList(start, Decimal) => format!(r#"<ol type="1" start="{start}">"#),
            OrderedList(start, LowerAlpha) => format!(r#"<ol type="a" start="{start}">"#),
            OrderedList(start, UpperAlpha) => format!(r#"<ol type="A" start="{start}">"#),
            OrderedList(start, LowerRoman) => format!(r#"<ol type="i" start="{start}">"#),
            OrderedList(start, UpperRoman) => format!(r#"<ol type="I" start="{start}">"#),
        }
    }

    fn closing_html_tag(&self) -> String {
        match self.list_type {
            ListType::OrderedList(_, _) => "</ol>",
            ListType::UnorderedList => "</ul>",
        }
        .to_string()
    }

    pub fn to_html(&self) -> String {
        let json_vec = self.to_html_vec();
        json!(json_vec).to_string()
    }

    fn to_html_vec(&self) -> Vec<Value> {
        let mut json_vec: Vec<Value> = vec![];
        json_vec.push(Value::String(self.opening_html_tag()));
        for item in &self.items {
            json_vec.extend(item.to_html_vec());
        }
        json_vec.push(Value::String(self.closing_html_tag()));
        json_vec
    }

    fn opening_latex_command(&self) -> String {
        use ListType::*;
        use OrderedType::*;
        match self.list_type {
            UnorderedList => "\\begin{itemize}\n".to_string(),
            OrderedList(start, Decimal) => {
                let mut string = format!(r#"\begin{{enumerate}}[start={start}, label=(\arabic*)]"#);
                string.push('\n');
                string
            }
            OrderedList(start, LowerAlpha) => {
                let mut string = format!(r#"\begin{{enumerate}}[start={start}, label=(\alph*)]"#);
                string.push('\n');
                string
            }
            OrderedList(start, UpperAlpha) => {
                let mut string = format!(r#"\begin{{enumerate}}[start={start}, label=(\Alph*)]"#);
                string.push('\n');
                string
            }
            OrderedList(start, LowerRoman) => {
                let mut string = format!(r#"\begin{{enumerate}}[start={start}, label=(\roman*)]"#);
                string.push('\n');
                string
            }
            OrderedList(start, UpperRoman) => {
                let mut string = format!(r#"\begin{{enumerate}}[start={start}, label=(\Roman*)]"#);
                string.push('\n');
                string
            }
        }
    }

    fn closing_latex_command(&self) -> String {
        match self.list_type {
            ListType::OrderedList(_, _) => "\\end{enumerate}\n",
            ListType::UnorderedList => "\\end{itemize}\n",
        }
        .to_string()
    }

    pub fn to_latex(&self) -> String {
        if self.max_depth() > 4 {
            eprintln!("List is too deep to be rendered in LaTeX");
            std::process::exit(0);
        }
        if self.jumps_multiple_levels() {
            eprintln!("List jumps multiple levels, which is not supported in LaTeX");
            std::process::exit(0);
        }
        let json_vec = self.to_latex_vec();
        json!(json_vec).to_string()
    }

    fn to_latex_vec(&self) -> Vec<Value> {
        let mut json_vec: Vec<Value> = vec![];
        json_vec.push(import!(r"\usepackage{enumitem}"));
        json_vec.push(Value::String(self.opening_latex_command()));
        for item in &self.items {
            json_vec.extend(item.to_latex_vec());
        }
        json_vec.push(Value::String(self.closing_latex_command()));
        json_vec
    }

    pub fn from_str(s: &str, spaces_per_indent: u64) -> Result<Self, InvalidListError> {
        if s.lines().count() == 0 || s.lines().next().unwrap().parse::<ListType>().is_err() {
            return Err(InvalidListError);
        }

        let mut lines: Vec<(String, ListType)> = vec![];

        for line in s.lines() {
            if let Ok(list_type) = line.parse::<ListType>() {
                lines.push((line.to_string(), list_type));
            } else {
                let last_index = lines.len() - 1;
                let last = lines.get_mut(last_index).unwrap();
                last.0.push('\n');
                last.0.push_str(line);
            }
        }

        let lines: Vec<(u64, ListType, String)> = lines
            .iter()
            .map(|(l, list_type)| {
                let level = l.chars().take_while(char::is_ascii_whitespace).count() as u64
                    / spaces_per_indent;
                (
                    level,
                    *list_type,
                    l.trim_start()
                        .chars()
                        .skip_while(|c| !c.is_ascii_whitespace())
                        .skip(1)
                        .collect::<String>(),
                )
            })
            .collect();

        let list_item = List::from_lines(&lines, 0, 0)?.0;

        if let ListItem::List(list) = list_item {
            Ok(list)
        } else {
            // This cannot happen but due to using from_lines recursively and having a
            // ListItem vec we need to unwrap the list here from the ListItem struct here.
            Err(InvalidListError)
        }
    }

    fn from_lines(
        lines: &Vec<(u64, ListType, String)>,
        curr_level: u64,
        start_index: usize,
    ) -> Result<(ListItem, usize), InvalidListError> {
        use std::cmp::Ordering;
        let mut items = vec![];
        let mut first_type = None;

        let mut i = start_index;
        while i < lines.len() {
            let (level, list_type, content) = &lines[i];
            match level.cmp(&curr_level) {
                Ordering::Less => break,
                Ordering::Equal => {
                    items.push(ListItem::Content(content.to_string()));
                    if first_type.is_none() {
                        first_type = Some(*list_type);
                    }
                }
                Ordering::Greater => {
                    let (item, new_i) = List::from_lines(lines, curr_level + 1, i)?;
                    i = new_i - 1;
                    items.push(item);
                }
            }
            i += 1;
        }

        Ok((
            ListItem::List(List {
                list_type: first_type.unwrap_or(ListType::UnorderedList),
                items,
            }),
            i,
        ))
    }
}

fn alpha_to_start(c: char) -> Option<u32> {
    let lowercase = c.is_ascii_lowercase().then_some(c as u8 - b'a' + 1);
    let uppercase = c.is_ascii_uppercase().then_some(c as u8 - b'A' + 1);
    lowercase.or(uppercase).map(u32::from)
}
