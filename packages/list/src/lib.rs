use serde_json::json;
use std::str::FromStr;

const MAX_DEPTH: usize = 255;

#[derive(Debug)]
pub struct NoListError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemType {
    Bullet,
    Decimal,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

impl ItemType {
    fn opening_tag(&self, level: usize) -> String {
        use ItemType::*;
        match self {
            Bullet => "<ul>".to_string(),
            Decimal => format!(r#"<ol type="1" start="{level}">"#),
            LowerAlpha => format!(r#"<ol type="a" start="{level}">"#),
            UpperAlpha => format!(r#"<ol type="A" start="{level}">"#),
            LowerRoman => format!(r#"<ol type="i" start="{level}">"#),
            UpperRoman => format!(r#"<ol type="I" start="{level}">"#),
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
        if *self == ItemType::Bullet {
            false
        } else {
            true
        }
    }
}

struct InvalidItem;

impl FromStr for ItemType {
    type Err = InvalidItem;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ItemType::*;
        match s {
            "-" => Ok(Bullet),
            "+" => Ok(Bullet),
            "*" => Ok(Bullet),
            "1." => Ok(Decimal),
            "1)" => Ok(Decimal),
            "(1)" => Ok(Decimal),
            "a." => Ok(LowerAlpha),
            "a)" => Ok(LowerAlpha),
            "(a)" => Ok(LowerAlpha),
            "A." => Ok(UpperAlpha),
            "A)" => Ok(UpperAlpha),
            "(A)" => Ok(UpperAlpha),
            "i." => Ok(LowerRoman),
            "i)" => Ok(LowerRoman),
            "(i)" => Ok(LowerRoman),
            "I." => Ok(UpperRoman),
            "I)" => Ok(UpperRoman),
            "(I)" => Ok(UpperRoman),
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

impl FromStr for List {
    type Err = NoListError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // If the first nonempty line is not a list item, return err
        s.lines()
            .skip_while(|&l| l == "")
            .next()
            .unwrap_or_else(|| "")
            .trim()
            .split_ascii_whitespace()
            .next()
            .unwrap_or_else(|| "")
            .parse::<ItemType>()
            .map_err(|_| NoListError)?;

        let items: Vec<ListItem> =
            s.lines()
                .skip_while(|&l| l == "")
                .fold(Vec::new(), |mut items, line| {
                    let start = line
                        .trim()
                        .split_ascii_whitespace()
                        .next()
                        .unwrap_or_else(|| "")
                        .parse::<ItemType>();

                    match start {
                        Ok(item_type) => {
                            let level = (line
                                .chars()
                                .take_while(|&x| x.is_ascii_whitespace())
                                .count()
                                / 4)
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
                            if line != "" {
                                last_item.content.push_str(line);
                            } else {
                                last_item.content.push('\n');
                            }
                            items[last_index] = last_item;
                        }
                    }
                    items
                });

        Ok(List { items })
    }
}

impl List {
    pub fn to_html(&self) -> String {
        if self.items.len() == 0 {
            return String::new();
        }
        let mut counters = vec![1; MAX_DEPTH];
        let (mut json_arr, mut tag_stack) = self.items.iter().fold(
            (json!([]), vec![]),
            |(mut json_arr, mut tag_stack), item| {
                while item.level != tag_stack.len() {
                    if item.level > tag_stack.len() {
                        tag_stack.push(item.item_type);
                        let opening_tag = item.item_type.opening_tag(counters[item.level]);
                        json_arr
                            .as_array_mut()
                            .unwrap()
                            .push(json!({"name": "raw", "data": opening_tag}));
                    } else {
                        let reset_level = tag_stack.len();
                        let closing_tag = tag_stack.pop().unwrap().closing_tag();
                        json_arr
                            .as_array_mut()
                            .unwrap()
                            .push(json!({"name": "raw", "data": closing_tag}));
                        *counters.get_mut(reset_level).unwrap() = 1;
                    }
                }
                let closing_tag = tag_stack.pop().unwrap().closing_tag();
                tag_stack.push(item.item_type);
                json_arr.as_array_mut().unwrap().extend(
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
            json_arr
                .as_array_mut()
                .unwrap()
                .push(json!({"name": "raw", "data": closing_tag}));
        }

        json_arr.to_string()
    }
}
