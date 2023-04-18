use calc_lib::{evaluate_with_defined, Definitions, Error, Functions};
use plotters::prelude::*;
use plotters::style::ShapeStyle;
use serde_json::json;

#[macro_export]
macro_rules! raw {
    ($expr:expr) => {
        json!({
            "name": "raw",
            "data": $expr
        })
    }
}

pub fn manifest() {
    print!(
        "{}",
        json!(
            {
            "name": "plot",
            "version": "0.1",
            "description": "This package provides graphical plotting.",
            "transforms": [
                {
                    "from": "plot",
                    "to": ["html"],
                    "description": "Plot a mathematical function",
                    "arguments": [
                        {
                            "name": "caption",
                            "description": "Caption for the plot.",
                            "default": "",
                        },
                        {
                            "name": "label",
                            "description": "Label to use for the plot, to be able to refer to it from the document.",
                            "default": "",
                        },
                        {
                            "name": "width",
                            "description": "Width of the plot. For HTML this is given as the ratio to the width of the surrounding figure tag (created automatically).",
                            "default": 1.0,
                            "type": "f64",
                        },
                        {
                            "name": "x_range",
                            "description": "The range of x-values that are plotted. This is given as two numbers with a space between.",
                            "default": "-20 20",
                        },
                        {
                            "name": "y_range",
                            "description": "The range of y-values that are plotted. This is given as two numbers with a space between.",
                            "default": "-20 20",
                        },
                        {
                            "name": "save",
                            "description": "The name of the SVG file that is saved. No file is saved if this argument is left empty.",
                            "default": "",
                        },
                        {
                            "name": "line_width",
                            "description": "The width of the line that is used in the plot.",
                            "type": "uint",
                            "default": 1,
                        },
                        {
                            "name": "line_color",
                            "description": "The color of the line that is used in the plot.",
                            "type": ["red", "blue", "green", "yellow"],
                            "default": "red",
                        },
                    ],
                },
            ]
            }
        )
    );
}

pub fn verify_function(function: &str, var_count: usize) -> bool {
    if function.is_empty() {
        return false;
    }

    let names = get_var_names(function);
    if names.len() > var_count {
        return false;
    }

    let mut defs = Definitions::new();
    for name in names {
        defs.register(name, 0)
    }

    // prefix with "0+" cause otherwise functions starting with "-" break
    // suspected calc_lib bugs: "+-x" does not subtract x, function args such as "2x" do not work
    let prefixed = format!("0+{function}");

    let calc_funcs = get_calc_funcs();
    // if error is DivByZero or NegativeExponent it is likely a valid function
    match evaluate_with_defined(prefixed, Some(&defs), Some(&calc_funcs)) {
        Ok(_) => true,
        Err(Error::DivByZero) | Err(Error::NegativeExponent) => true,
        _ => false,
    }
}

pub fn get_calc_funcs() -> Functions<'static> {
    let mut funcs = Functions::default();

    funcs.register("mod", |args| match args.len() {
        2 => Ok(args[1] % args[0]),
        _ => Err(Error::arg_count("mod", 2, args.len())),
    });

    funcs
}

// make sure this is up to date and matches get_calc_funcs
// also assumes that all variables are one character
fn get_var_names(function: &str) -> Vec<String> {
    let replaced = function
        .replace("sin", "")
        .replace("cos", "")
        .replace("tan", "")
        .replace("sqrt", "")
        .replace("log", "")
        .replace("mod", "");

    let mut vars = vec![];
    for c in replaced.chars() {
        if c.is_alphabetic() {
            let s = c.to_string();
            if !vars.contains(&s) {
                vars.push(s);
            }
        }
    }
    vars
}

pub fn eval_function(function: &str, vars: Vec<f32>, calc_funcs: &Functions) -> Result<f64, Error> {
    let mut defs = Definitions::new();
    let names = get_var_names(function);
    let mut names_iter = names.iter();
    let mut vars_iter = vars.iter();

    while let (Some(name), Some(&var)) = (names_iter.next(), vars_iter.next()) {
        defs.register(name, var);
    }

    // prefix with "0+" cause otherwise functions starting with "-" break
    // suspected calc_lib bugs: "+-x" does not subtract x, function args such as "2x" do not work
    let prefixed = format!("0+{function}");
    evaluate_with_defined(prefixed, Some(&defs), Some(calc_funcs))
}

pub fn get_shape_style(color_str: &str, width: u32) -> ShapeStyle {
    let color = match color_str {
        "red" => RED.into(),
        "blue" => BLUE.into(),
        "green" => GREEN.into(),
        "yellow" => YELLOW.into(),
        _ => RED.into(),
    };
    ShapeStyle {
        color,
        filled: false,
        stroke_width: width,
    }
}
