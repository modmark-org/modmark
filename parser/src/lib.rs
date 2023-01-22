// Just some placeholder code. We will decide how to structure
// our parser and what internal representation to use later :)

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Data(String),
    Node {
        name: String,
        attributes: HashMap<String, String>,
        children: Vec<Element>,
    },
}

impl Element {
    pub fn tree_string(&self, include_attributes: bool) -> String {
        pretty_rows(&self, include_attributes).join("\n")
    }
}

fn pretty_rows(element: &Element, include_attributes: bool) -> Vec<String> {
    let indent = "  ";
    let mut strs = vec![];

    match element {
        Element::Data(str) => strs.push(format!(r#""{str}""#)),
        Element::Node { name, attributes, children } =>
            {
                strs.push(format!("{name} {{"));
                if attributes.is_empty() {
                    strs.push(format!("{indent}attributes: {{ <empty> }}"));
                } else if include_attributes {
                    strs.push(format!("{indent}attributes: {{"));

                    attributes.iter().for_each(
                        |(k, v)|
                            strs.push(format!(r#"{indent}{indent}"{k}": "{v}""#))
                    );

                    strs.push(format!("{indent}}}"));
                } else {
                    strs.push(format!("{indent}attributes: {{ < {len} attributes > }}", len = &attributes.len().to_string()))
                }

                if children.is_empty() {
                    strs.push(format!("{indent}children: [ none ]"));
                } else {
                    strs.push(format!("{indent}children: ["));

                    children.into_iter().for_each(|c|
                        pretty_rows(&c, include_attributes)
                            .iter()
                            .for_each(|s|
                                strs.push(format!("{indent}{indent}{s}"))
                            )
                    );

                    strs.push(format!("{indent}]"));
                }
                strs.push("}".to_string());
            }
    }

    return strs;
}

pub fn parse(source: &str) -> Element {
    let mut doc: Element = Element::Node {
        name: "Document".into(),
        attributes: HashMap::new(),
        children: vec![],
    };

    let default_paragraph: Element = Element::Node {
        name: "Paragraph".into(),
        attributes: HashMap::new(),
        children: vec![],
    };

    let mut current_paragraph: Element = default_paragraph.clone();

    source.lines().for_each(|str|
        if str.trim().is_empty() {
            match &mut doc {
                Element::Node { name: _, attributes: _, children } =>
                    children.push(current_paragraph.clone()),
                _ => {}
            }
            current_paragraph = default_paragraph.clone();
        } else {
            match &mut current_paragraph {
                Element::Node { name: _, attributes: _, children } =>
                    children.push(Element::Data(str.clone().into())),
                _ => { () }
            }
        }
    );

    match &mut doc {
        Element::Node { name: _, attributes: _, children } =>
            children.push(current_paragraph),
        _ => {}
    }

    return doc;
}
