use std::str::FromStr;

use serde_json::json;

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
    fn opening_tag(&self) -> String {
        use ItemType::*;
        match self {
            Bullet => "<ul>",
            Decimal => r#"<ol type="1">"#,
            LowerAlpha => r#"<ol type="a">"#,
            UpperAlpha => r#"<ol type="A">"#,
            LowerRoman => r#"<ol type="i">"#,
            UpperRoman => r#"<ol type="I">"#,
        }
        .to_string()
    }

    fn closing_tag(&self) -> String {
        use ItemType::*;
        match self {
            Bullet => "</ul>",
            Decimal => "</ol>",
            LowerAlpha => "</ol>",
            UpperAlpha => "</ol>",
            LowerRoman => "</ol>",
            UpperRoman => "</ol>",
        }
        .to_string()
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
                                / 2)
                                + 1;
                            items.push(ListItem {
                                item_type,
                                content: line
                                    .chars()
                                    .skip_while(|&x| x.is_ascii_whitespace())
                                    .collect::<String>()
                                    .split_ascii_whitespace()
                                    .skip(1)
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
        let initial_arr = json!([
            {"name": "raw", "data": self.items[0].item_type.opening_tag()},
            {"name": "raw", "data": "<li>"},
            {"name": "block_content", "data": self.items[0].content},
            {"name": "raw", "data": "</li>"}
        ]);
        let (mut json_arr, mut tag_stack, _) = self.items.iter().skip(1).fold(
            (
                initial_arr,
                vec![self.items[0].item_type],
                self.items[0].item_type,
            ),
            |(mut json_arr, mut tag_stack, last_type), item| {
                let mut flag = false;
                while item.level != tag_stack.len() {
                    if item.level > tag_stack.len() {
                        tag_stack.push(item.item_type);
                        let opening_tag = item.item_type.opening_tag();
                        json_arr
                            .as_array_mut()
                            .unwrap()
                            .push(json!({"name": "raw", "data": opening_tag}));
                        flag = true;
                    } else {
                        let closing_tag = tag_stack.pop().unwrap().closing_tag();
                        json_arr
                            .as_array_mut()
                            .unwrap()
                            .push(json!({"name": "raw", "data": closing_tag}));
                    }
                }
                if last_type == item.item_type || flag {
                    json_arr.as_array_mut().unwrap().extend(
                        json!([
                            {"name": "raw", "data": "<li>"},
                            {"name": "block_content", "data": item.content},
                            {"name": "raw", "data": "</li>"}
                        ])
                        .as_array()
                        .unwrap()
                        .clone(),
                    )
                } else {
                    let closing_tag = tag_stack.pop().unwrap().closing_tag();
                    json_arr.as_array_mut().unwrap().extend(
                        json!([
                            {"name": "raw", "data": closing_tag},
                            {"name": "raw", "data": item.item_type.opening_tag()},
                            {"name": "raw", "data": "<li>"},
                            {"name": "block_content", "data": item.content},
                            {"name": "raw", "data": "</li>"}
                        ])
                        .as_array()
                        .unwrap()
                        .clone(),
                    )
                }
                (json_arr, tag_stack, item.item_type)
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

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn it_works() {
        let test: Value = serde_json::from_str(
            &include_str!("example.txt")
                .parse::<List>()
                .unwrap()
                .to_html(),
        )
        .unwrap();
        println!("{:#?}", test);
        assert_eq!(1, 1);
    }
}
