use plot::manifest;
use plot2d::transform_plot_2d;
use plotlist::transform_plot_list;
use std::env;
use std::io::{self, Read};

mod plot2d;
mod plot3d;
mod plotlist;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let action = &args[0];

    match action.as_str() {
        "manifest" => manifest(),
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

    match from {
        "plot" => transform_plot_2d(input),
        "plot_list" => transform_plot_list(input),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}
