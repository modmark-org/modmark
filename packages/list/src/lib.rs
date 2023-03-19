use serde_json::{json, Value};
use std::str::FromStr;

const MAX_DEPTH: usize = 255;

#[derive(Debug)]
pub struct NoListError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// Options in ordered types represent if they have a specified start value
enum ItemType {
    Bullet,
    Decimal(Option<u32>),
    LowerAlpha(Option<u32>),
    UpperAlpha(Option<u32>),
    LowerRoman(Option<u32>),
    UpperRoman(Option<u32>),
}

impl ItemType {
    fn opening_tag(&self, start: u32) -> String {
        use ItemType::*;
        match self {
            Bullet => "<ul>".to_string(),
            Decimal(_) => format!(r#"<ol type="1" start="{start}">"#),
            LowerAlpha(_) => format!(r#"<ol type="a" start="{start}">"#),
            UpperAlpha(_) => format!(r#"<ol type="A" start="{start}">"#),
            LowerRoman(_) => format!(r#"<ol type="i" start="{start}">"#),
            UpperRoman(_) => format!(r#"<ol type="I" start="{start}">"#),
        }
    }

    fn closing_tag(&self) -> String {
        if *self == ItemType::Bullet {
            "</ul>"
        } else {
            "</ol>"
        }
        .to_string()
    }

    fn is_ordered(&self) -> bool {
        *self != ItemType::Bullet
    }

    fn get_start_preference(&self) -> Option<u32> {
        use ItemType::*;
        match *self {
            Bullet => None,
            Decimal(start) => start,
            LowerAlpha(start) => start,
            UpperAlpha(start) => start,
            LowerRoman(start) => start,
            UpperRoman(start) => start,
        }
    }
}

struct InvalidItem;

impl FromStr for ItemType {
    type Err = InvalidItem;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ItemType::*;
        let start_str = if s.starts_with('(') && s.ends_with(')') {
            &s[1..s.len() - 1]
        } else if s.ends_with(')') || s.ends_with('.') {
            &s[0..s.len() - 1]
        } else {
            ""
        };

        if !start_str.is_empty() {
            if let Ok(start) = start_str.parse::<u32>() {
                return Ok(Decimal(Some(start)));
            } else if let Some(start) = roman::from(&start_str.to_ascii_uppercase()) {
                if start_str.chars().next().unwrap().is_ascii_lowercase() {
                    return Ok(LowerRoman(Some(start.unsigned_abs())));
                } else {
                    return Ok(UpperRoman(Some(start.unsigned_abs())));
                }
            } else if start_str.chars().next().unwrap().is_ascii_alphabetic() {
                let c = start_str.chars().next().unwrap();
                let order = alpha_to_start(&c);
                if let Some(start) = order {
                    if c.is_ascii_lowercase() {
                        return Ok(LowerAlpha(Some(start)));
                    } else {
                        return Ok(UpperAlpha(Some(start)));
                    }
                }
            }
        }

        match s {
            "-" => Ok(Bullet),
            "+" => Ok(Bullet),
            "*" => Ok(Bullet),
            "1." => Ok(Decimal(None)),
            "1)" => Ok(Decimal(None)),
            "(1)" => Ok(Decimal(None)),
            "a." => Ok(LowerAlpha(None)),
            "a)" => Ok(LowerAlpha(None)),
            "(a)" => Ok(LowerAlpha(None)),
            "A." => Ok(UpperAlpha(None)),
            "A)" => Ok(UpperAlpha(None)),
            "(A)" => Ok(UpperAlpha(None)),
            "i." => Ok(LowerRoman(None)),
            "i)" => Ok(LowerRoman(None)),
            "(i)" => Ok(LowerRoman(None)),
            "I." => Ok(UpperRoman(None)),
            "I)" => Ok(UpperRoman(None)),
            "(I)" => Ok(UpperRoman(None)),
            _ => Err(InvalidItem),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ListItem {
    item_type: ItemType,
    content: String,
    level: usize,
}

#[derive(Debug)]
pub struct List {
    pub items: Vec<ListItem>,
}

impl List {
    pub fn to_html(&self) -> String {
        if self.items.is_empty() {
            return String::new();
        }
        let mut counters: Vec<u32> = vec![1; MAX_DEPTH];
        let (mut json_vec, mut tag_stack) = self.items.iter().fold(
            (Vec::<Value>::new(), vec![]),
            |(mut json_arr, mut tag_stack), item| {
                // new_level is used to check if the start preference
                // of a item can be used
                let mut new_level = false;
                while item.level != tag_stack.len() {
                    if item.level > tag_stack.len() {
                        new_level = true;
                        tag_stack.push(item.item_type);
                        let opening_tag = item.item_type.opening_tag(counters[item.level]);
                        json_arr.push(json!({"name": "raw", "data": opening_tag}));
                    } else {
                        let reset_level = tag_stack.len();
                        let closing_tag = tag_stack.pop().unwrap().closing_tag();
                        json_arr.push(json!({"name": "raw", "data": closing_tag}));
                        *counters.get_mut(reset_level).unwrap() = 1;
                    }
                }
                let closing_tag = tag_stack.pop().unwrap().closing_tag();
                tag_stack.push(item.item_type);
                if new_level {
                    if let Some(start) = item.item_type.get_start_preference() {
                        *counters.get_mut(item.level).unwrap() = start;
                    }
                }
                json_arr.extend(
                    json!([
                        {"name": "raw", "data": closing_tag},
                        {"name": "raw", "data": item.item_type.opening_tag(counters[item.level])},
                        {"name": "raw", "data": "<li>"},
                        {"name": "block_content", "data": item.content},
                        {"name": "raw", "data": "</li>"}
                    ])
                    .as_array()
                    .unwrap()
                    .clone(),
                );
                if item.item_type.is_ordered() {
                    *counters.get_mut(item.level).unwrap() += 1;
                }
                (json_arr, tag_stack)
            },
        );

        while let Some(item_type) = tag_stack.pop() {
            let closing_tag = item_type.closing_tag();
            json_vec.push(json!({"name": "raw", "data": closing_tag}));
        }

        json!(json_vec).to_string()
    }

    pub fn from_str(s: &str, indent: u32) -> Result<Self, NoListError> {
        // If the first nonempty line is not a list item, return err
        s.lines()
            .find(|&l| !l.is_empty())
            .unwrap_or("")
            .trim()
            .split_ascii_whitespace()
            .next()
            .unwrap_or("")
            .parse::<ItemType>()
            .map_err(|_| NoListError)?;

        let items: Vec<ListItem> =
            s.lines()
                .skip_while(|&l| l.is_empty())
                .fold(Vec::new(), |mut items, line| {
                    let start = line
                        .trim()
                        .split_ascii_whitespace()
                        .next()
                        .unwrap_or("")
                        .parse::<ItemType>();

                    match start {
                        Ok(item_type) => {
                            let level = (line
                                .chars()
                                .take_while(|&x| x.is_ascii_whitespace())
                                .count()
                                / indent as usize)
                                + 1;
                            items.push(ListItem {
                                item_type,
                                content: line
                                    .chars()
                                    .skip_while(|&x| x.is_ascii_whitespace())
                                    .skip_while(|&x| !x.is_ascii_whitespace())
                                    .skip_while(|&x| x.is_ascii_whitespace())
                                    .collect::<String>(),
                                level,
                            });
                        }
                        Err(_) => {
                            let last_index = items.len() - 1;
                            let mut last_item = items[last_index].clone();
                            last_item.content.push('\n');
                            last_item.content.push_str(line);
                            items[last_index] = last_item;
                        }
                    }
                    items
                });

        Ok(List { items })
    }
}

fn alpha_to_start(c: &char) -> Option<u32> {
    match c {
        'a' => Some(1),
        'b' => Some(2),
        'c' => Some(3),
        'd' => Some(4),
        'e' => Some(5),
        'f' => Some(6),
        'g' => Some(7),
        'h' => Some(8),
        'i' => Some(9),
        'j' => Some(10),
        'k' => Some(11),
        'l' => Some(12),
        'm' => Some(13),
        'n' => Some(14),
        'o' => Some(15),
        'p' => Some(16),
        'q' => Some(17),
        'r' => Some(18),
        's' => Some(19),
        't' => Some(20),
        'u' => Some(21),
        'v' => Some(22),
        'w' => Some(23),
        'x' => Some(24),
        'y' => Some(25),
        'z' => Some(26),
        'A' => Some(1),
        'B' => Some(2),
        'C' => Some(3),
        'D' => Some(4),
        'E' => Some(5),
        'F' => Some(6),
        'G' => Some(7),
        'H' => Some(8),
        'I' => Some(9),
        'J' => Some(10),
        'K' => Some(11),
        'L' => Some(12),
        'M' => Some(13),
        'N' => Some(14),
        'O' => Some(15),
        'P' => Some(16),
        'Q' => Some(17),
        'R' => Some(18),
        'S' => Some(19),
        'T' => Some(20),
        'U' => Some(21),
        'V' => Some(22),
        'W' => Some(23),
        'X' => Some(24),
        'Y' => Some(25),
        'Z' => Some(26),
        _ => None,
    }
}
