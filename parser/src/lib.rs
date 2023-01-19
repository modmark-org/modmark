// Just some placeholder code. We will decide how to structure
// our parser and what internal representation to use later :)

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Document(Box<Self>),
    Paragraph(Box<Self>),
    Text(String),
}

pub fn parse(source: &str) -> Node {
    Node::Document(Box::new(Node::Paragraph(Box::new(Node::Text(
        source.to_string(),
    )))))
}
