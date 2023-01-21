use parser::Element;

// It is still somewhat unclear how we want to structure this part of the code base.
// But basically we would transform the document tree by evaluating all of the nodes
// in a wasm runtime.

pub fn eval(document: &Element) -> String {
    // Note that the real program of course won't be hard-coded to transpile into html like this
    match document {
        Element::Document(content) => format!("<main>{}</main>", eval(content)),
        Element::Paragraph(content) => format!("<p>{}</p>", eval(content)),
        Element::Text(content) => content.to_owned(),
    }
}
