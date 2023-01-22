use parser::Element;
use std::collections::HashMap;

// It is still somewhat unclear how we want to structure this part of the code base.
// But basically we would transform the document tree by evaluating all of the nodes
// in a wasm runtime.

pub fn eval(document: &Element) -> String {
    // Note that the real program of course won't be hard-coded to transpile into html like this
    return elem_to_html(&document);
}

// Example function turning an element to html
fn elem_to_html(elem: &Element) -> String {
    match elem {
        Element::Data(str) => str.to_string(),
        Element::Node { name, attributes, children } => {
            inode_to_html(&name, &attributes, &children)
        }
    }
}

// Example turning inode (non-leaf node) to html, to make above function cleaner
fn inode_to_html(name: &str, attributes: &HashMap<String, String>, children: &Vec<Element>) -> String {
    match name {
        "Document" => format!("<main>\n{}\n</main>",
                              children.iter()
                                  .map(|e| elem_to_html(&e))
                                  .collect::<Vec<String>>()
                                  .join("\n")
        ),
        "Paragraph" => format!("<p>\n{}\n</p>",
                               children.iter()
                                   .map(|e| elem_to_html(&e))
                                   .collect::<Vec<String>>()
                                   .join("\n")),
        &_ => panic!("No inode=>html for node '{name}'")
    }
}
