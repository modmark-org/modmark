use std::env;
use std::io;

fn main() {
    println!("Hello, world!");
}

#[no_mangle]
pub fn name() {
    println!("MyCoolBoldModule 😎")
}

#[no_mangle]
pub fn version() {
    println!("0.1.1");
}

#[no_mangle]
pub fn test_envs() {
    let args: Vec<String> = env::args().collect();

    println!("{}", args[1]);
}

#[no_mangle]
pub fn transforms() {
    println!("bold->html");
    println!("bold->latex");
}

#[no_mangle]
pub fn attributes() {
    println!("attribute-five");
    println!("list-symbol=bullet");
    println!("bullet-spacing=1");
}
