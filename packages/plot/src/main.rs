use plots::{get_function_svg, get_list_svg};
use std::env;
use std::io::{self, Read};
use utils::*;
use utils::{print_manifest, print_svg_html};

mod eval;
mod plots;
mod utils;

// TODO: Style axes better
// TODO: Comment code

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let action = &args[0];

    match action.as_str() {
        "manifest" => print_manifest(),
        "transform" => transform(&args[1], &args[2]),
        other => {
            eprintln!("Invalid action {other}")
        }
    }
}

fn transform(from: &str, _to: &str) {
    let input = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };

    let mut ctx = match PlotContext::new(input) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    let res = match from {
        "plot" => get_function_svg(&mut ctx),
        "plot-list" => get_list_svg(&mut ctx),
        other => {
            eprintln!("Package does not support {other}");
            return;
        }
    };
    let svg = match res {
        Ok(svg) => svg,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    print_svg_html(svg, &ctx)
}
