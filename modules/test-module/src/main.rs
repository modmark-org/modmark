use std::{env, io};

#[no_mangle]
pub fn name() {
    println!("My cool utils 🥑");
}

#[no_mangle]
pub fn version() {
    println!("1");
}

#[no_mangle]
pub fn transforms() {
    /*
    [verbatim] -> tex html
    foo       - An example of a required positional argument
    ident = 4 - The number of spaces to indent

    [table] -> table

    table -> html latex

    row - html latex
    */

    println!("[verbatim] -> html latex");
    println!("foo - An example of a required postional argument");
    println!("ident = 4 - The number of spaces to indent the verbatim block");
    println!("");

    println!("[table] -> table");
    println!("");

    println!("table -> html latex");
    println!("");

    println!("row -> html latex");
    println!("");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let node_name = &args[0];

    let mut body = String::new();
    io::stdin().read_line(&mut body).unwrap();

    match node_name.as_str() {
        "[verbatim]" => todo!(),
        "[table]" => todo!(),
        "row" => todo!(),
        "table" => todo!(),
        other => eprintln!("This node does not support node conversions from {other}"),
    }
}
