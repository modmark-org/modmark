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
    println!("[verbatim indent=4] -> html latex");
    println!("[table] -> table");
    println!("table -> html latex");
    println!("row -> html latex");
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

