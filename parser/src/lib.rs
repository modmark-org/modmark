// Just some placeholder code. We will decide how to structure
// our parser and what internal representation to use later :)

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Document(Box<Self>),
    Paragraph(Box<Self>),
    Text(String),
}

pub fn parse(source: &str) -> Element {
    Element::Document(Box::new(Element::Paragraph(Box::new(Element::Text(
        source.to_string(),
    )))))
}
