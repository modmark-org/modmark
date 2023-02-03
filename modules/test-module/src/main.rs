use std::{env, io};

#[no_mangle]
pub fn name() {
    println!("Module test");
}

#[no_mangle]
pub fn version() {
    println!("1");
}

#[no_mangle]
pub fn transforms() {
    println!("[table] -> table");
    println!("border = black - What color the border should be");
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
        "[table]" => todo!(),
        "row" => todo!(),
        "table" => todo!(),
        other => eprintln!("This node does not support node conversions from {other}"),
    }
}
