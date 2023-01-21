use core::eval;
use parser::parse;
use std::{env, fs};

// We would likely want to use "clap" and its derive feature to describe
// the command-line interface with a declarative struct

fn main() {
    let path: String = env::args().skip(1).take(1).collect();
    let source = fs::read_to_string(path).expect("Failed to read file");

    let document = parse(&source);
    let output = eval(&document);

    println!("{output}");
}
