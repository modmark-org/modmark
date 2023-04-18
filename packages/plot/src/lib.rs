use evalexpr::*;
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
                            "name": "color",
                            "description": "The color that is used in the plot.",
                            "type": ["red", "blue", "green", "yellow"],
                            "default": "red",
                        },
                    ],
                },
                {
                    "from": "plot_list",
                    "to": ["html"],
                    "description": "Plot a set of (x,y) values. The values should be placed on separate lines, and the x-y pair should space-separated.",
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
                            "name": "color",
                            "description": "The color that is used in the plot.",
                            "type": ["red", "blue", "green", "yellow"],
                            "default": "red",
                        },
                        {
                            "name": "connect",
                            "description": "Decides if lines will be drawn between points.",
                            "type": ["false", "true"],
                            "default": "true",
                        },
                        {
                            "name": "point_size",
                            "description": "The diameter of each plotted point, given in pixels.",
                            "type": "uint",
                            "default": 3,
                        },
                    ],
                },
            ]
            }
        )
    );
}

fn value_to_float(v: &Value) -> Option<f64> {
    match v {
        Value::Float(f) => Some(*f),
        Value::Int(i) => Some(*i as f64),
        _ => None,
    }
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

pub fn new_function_context() -> HashMapContext {
    let mut ctx = HashMapContext::new();
    ctx.set_function(
        String::from("sin"),
        Function::new(|arg| match value_to_float(arg) {
            Some(float) => Ok(Value::Float(float.sin())),
            None => Err(EvalexprError::expected_float(arg.clone())),
        }),
    )
    .unwrap();
    ctx.set_function(
        String::from("cos"),
        Function::new(|arg| match value_to_float(arg) {
            Some(float) => Ok(Value::Float(float.cos())),
            None => Err(EvalexprError::expected_float(arg.clone())),
        }),
    )
    .unwrap();
    ctx.set_function(
        String::from("tan"),
        Function::new(|arg| match value_to_float(arg) {
            Some(float) => Ok(Value::Float(float.tan())),
            None => Err(EvalexprError::expected_float(arg.clone())),
        }),
    )
    .unwrap();
    ctx.set_function(
        String::from("sqrt"),
        Function::new(|arg| match value_to_float(arg) {
            Some(float) => Ok(Value::Float(float.sqrt())),
            None => Err(EvalexprError::expected_float(arg.clone())),
        }),
    )
    .unwrap();
    ctx.set_function(
        String::from("log"),
        Function::new(|arg| {
            let args = arg.as_tuple()?;
            match (value_to_float(&args[0]), value_to_float(&args[1])) {
                (Some(a), Some(b)) => Ok(Value::Float(b.log(a))),
                (_, _) => Err(EvalexprError::expected_float(arg.clone())),
            }
        }),
    )
    .unwrap();
    ctx.set_function(
        String::from("mod"),
        Function::new(|arg| {
            let args = arg.as_tuple()?;
            match (value_to_float(&args[0]), value_to_float(&args[1])) {
                (Some(a), Some(b)) => Ok(Value::Float(b % a)),
                (_, _) => Err(EvalexprError::expected_float(arg.clone())),
            }
        }),
    )
    .unwrap();
    ctx
}

fn update_function_context(ctx: &mut HashMapContext, names: &[String], values: &[f32]) {
    let mut names_iter = names.iter();
    let mut values_iter = values.iter();
    while let (Some(name), Some(&value)) = (names_iter.next(), values_iter.next()) {
        // only values from verify_function float ranges will be used here, fine to unwrap
        ctx.set_value(name.into(), (value as f64).into()).unwrap();
    }
}

pub fn verify_function(ctx: &mut HashMapContext, function: &str, var_count: usize) -> bool {
    if function.is_empty() {
        return false;
    }

    let names = get_var_names(function);
    if names.len() > var_count {
        return false;
    }

    let values = vec![0.0; names.len()];
    update_function_context(ctx, &names, &values);

    eval_float_with_context_mut(function, ctx).is_ok()
}

pub fn eval_function(
    ctx: &mut HashMapContext,
    function: &str,
    values: Vec<f32>,
) -> EvalexprResult<f64> {
    let names = get_var_names(function);
    update_function_context(ctx, &names, &values);
    eval_float_with_context_mut(function, ctx)
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
        filled: true,
        stroke_width: width,
    }
}
