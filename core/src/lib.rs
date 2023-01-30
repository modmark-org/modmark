use parser::{Element, ModuleArguments};
use std::collections::HashMap;

// It is still somewhat unclear how we want to structure this part of the code base.
// But basically we would transform the document tree by evaluating all of the nodes
// in a wasm runtime.

pub fn eval(document: &Element) -> String {
    // Note that the real program of course won't be hard-coded to transpile into html like this
    elem_to_html(document)
}

// Example function turning an element to html
fn elem_to_html(elem: &Element) -> String {
    match elem {
        Element::Data(str) => str.to_string(),
        Element::Node {
            name,
            attributes,
            children,
        } => inode_to_html(name, attributes, children),
        Element::ModuleInvocation { name, args, body, one_line } => invoke_to_html(name, args, body)
    }
}

fn invoke_to_html(
    name: &str,
    args: &Option<ModuleArguments>,
    body: &String
) -> String {
    "".to_string()
}

// Example turning inode (non-leaf node) to html, to make above function cleaner
fn inode_to_html(
    name: &str,
    _attributes: &HashMap<String, String>,
    children: &[Element],
) -> String {
    match name {
        "Document" => format!(
            "<main>\n{}\n</main>",
            children
                .iter()
                .map(elem_to_html)
                .collect::<Vec<String>>()
                .join("\n")
        ),
        "Paragraph" => format!(
            "<p>\n{}\n</p>",
            children
                .iter()
                .map(elem_to_html)
                .collect::<Vec<String>>()
                .join("\n")
        ),
        &_ => panic!("No inode=>html for node '{name}'"),
    }
}
