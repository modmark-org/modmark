fn main() {
    println!("Hello, world!");
}

#[no_mangle]
pub fn name() {
    println!("Bold")
}

#[no_mangle]
pub fn version() {
    println!("0.0.1");
}

fn transforms() {
    println!("bold -> html");
}