use base64::{engine::general_purpose, Engine as _};
use calc_lib::{evaluate_with_defined, Definitions, Error, Functions};
use plotters::prelude::*;
use plotters::style::ShapeStyle;
use serde_json::{json, Value};
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};

macro_rules! raw {
    ($expr:expr) => {
        json!({
            "name": "raw",
            "data": $expr
        })
    }
}

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
        "plot" => transform_plot(input),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_plot(input: Value) {
    let caption = input["arguments"]["caption"].as_str().unwrap();
    let label = input["arguments"]["label"].as_str().unwrap();
    let width = input["arguments"]["width"]
        .as_f64()
        .unwrap()
        .clamp(0.0, f64::MAX);

    let function = input["data"].as_str().unwrap().trim();
    let x_range = input["arguments"]["x_range"].as_str().unwrap();
    let y_range = input["arguments"]["y_range"].as_str().unwrap();
    let save = input["arguments"]["save"].as_str().unwrap();

    let svg = match get_svg(function, x_range, y_range) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let encoded: String = general_purpose::STANDARD_NO_PAD.encode(&svg);
    let src = format!("data:image/svg+xml;base64,{encoded}");
    let percentage = (width * 100.0).round() as i32;
    let style = format!("style=\"width:{percentage}%\"");
    let img_str = format!("<img src=\"{src}\" {style} ");

    let mut v = vec![];
    v.push(raw!("<figure>\n"));
    v.push(raw!(img_str));
    if !label.is_empty() {
        v.push(raw!("id=\""));
        v.push(json!({"name": "__text", "data": label}));
        v.push(raw!("\""));
    }
    v.push(raw!("/>\n"));
    if !caption.is_empty() {
        v.push(raw!("<figcaption>"));
        v.push(json!({"name": "__text", "data": caption}));
        v.push(raw!("</figcaption>\n"));
    }
    v.push(raw!("</figure>\n"));

    print!("{}", json!(v));

    if !save.is_empty() {
        if let Ok(mut file) = File::create(save) {
            write!(file, "{}", svg).unwrap();
        } else {
            eprintln!("Could not open the specified file.");
        }
    }
}

fn get_svg(function: &str, x_range: &str, y_range: &str) -> Result<String, String> {
    // string where svg is stored
    let mut buf = String::new();
    // scope to drop root before buf is needed again
    {
        // TODO: don't do this, this allows 3 numbers in x_range and 1 in y_range and so on
        let ranges = format!("{x_range} {y_range}");
        let bounds = ranges
            .split(' ')
            .filter_map(|v| v.parse::<f32>().ok())
            .collect::<Vec<f32>>();
        if bounds.len() != 4 {
            return Err("Invalid input for axis ranges.".to_string());
        }

        let x_fr = bounds[0];
        let x_to = bounds[1];
        let y_fr = bounds[2];
        let y_to = bounds[3];

        // quotient to scale range of points
        let q = 100.0 / (x_to - x_fr);
        let calc_funcs = get_calc_funcs();

        // verify that function is valid
        if !verify_function(function, &calc_funcs) {
            return Err("Invalid function.".to_string());
        }

        // create a line series of 200 points (half of svg width), scale the points to x_range
        // and filter out points that lie outside the axis ranges
        let line_series = LineSeries::new(
            (-100..=100)
                .map(|x| x as f32 / q + x_fr)
                .map(|x| (x, eval_function(function, x, &calc_funcs)))
                .filter(|(x, y)| &x_fr <= x && x <= &x_to && &y_fr <= y && y <= &y_to),
            ShapeStyle {
                color: RED.into(),
                filled: true,
                stroke_width: 2,
            },
        );

        let root = SVGBackend::with_string(&mut buf, (400, 400)).into_drawing_area();
        let mut chart = ChartBuilder::on(&root)
            .margin(10)
            .set_left_and_bottom_label_area_size(30)
            .build_cartesian_2d(x_fr..x_to, y_fr..y_to)
            .map_err(|_| "Failed to build coordinate system.".to_string())?;
        chart
            .configure_mesh()
            .max_light_lines(0)
            .draw()
            .map_err(|_| "Failed to draw coordinate system.".to_string())?;
        chart
            .draw_series(line_series)
            .map_err(|_| "Failed to plot the function.".to_string())?
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x, y)], RED));
        root.present()
            .map_err(|_| "Failed to create SVG.".to_string())?;
    }
    Ok(buf)
}

fn verify_function(function: &str, calc_funcs: &Functions) -> bool {
    if function.is_empty() {
        return false;
    }
    // TODO: find way to allow arbitrary variables
    let mut defs = Definitions::new();
    defs.register("x", 0);
    defs.register("y", 0);
    defs.register("z", 0);

    // prefix with "0+" cause otherwise functions starting with "-" break
    // suspected calc_lib bugs: "+-x" does not subtract x, function args such as "2x" do not work
    let prefixed = format!("0+{function}");

    // if error is DivByZero or NegativeExponent it is likely a valid function
    match evaluate_with_defined(prefixed, Some(&defs), Some(calc_funcs)) {
        Ok(_) => true,
        Err(Error::DivByZero) | Err(Error::NegativeExponent) => true,
        _ => false,
    }
}

fn eval_function(function: &str, x: f32, calc_funcs: &Functions) -> f32 {
    // TODO: find way to allow arbitrary variables
    let mut defs = Definitions::new();
    defs.register("x", x);
    defs.register("y", x);
    defs.register("z", x);

    // prefix with "0+" cause otherwise functions starting with "-" break
    // suspected calc_lib bugs: "+-x" does not subtract x, function args such as "2x" do not work
    let prefixed = format!("0+{function}");
    match evaluate_with_defined(prefixed, Some(&defs), Some(calc_funcs)) {
        Ok(val) => val as f32,
        _ => f32::MAX, // evaluation is undefined, f32::MAX will be outside of y-range and therefore not show up in plot
    }
}

fn get_calc_funcs() -> Functions<'static> {
    let mut funcs = Functions::new();
    funcs.register("log", |args| match args.len() {
        1 => Ok(args[0].log(10.0)),
        2 => Ok(args[0].log(args[1])),
        _ => Err(Error::arg_count("log", 2, args.len())),
    });

    funcs.register("sin", |args| match args.len() {
        1 => Ok(args[0].sin()),
        _ => Err(Error::arg_count("sin", 1, args.len())),
    });

    funcs.register("cos", |args| match args.len() {
        1 => Ok(args[0].cos()),
        _ => Err(Error::arg_count("cos", 1, args.len())),
    });

    funcs.register("tan", |args| match args.len() {
        1 => Ok(args[0].sin()),
        _ => Err(Error::arg_count("tan", 1, args.len())),
    });

    funcs.register("mod", |args| match args.len() {
        2 => Ok(args[0] % args[1]),
        _ => Err(Error::arg_count("mod", 2, args.len())),
    });

    funcs
}

fn manifest() {
    print!(
        "{}",
        json!(
            {
            "name": "plot",
            "version": "0.1",
            "description": "This package supports plotting of mathematical functions.",
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
                    ],
                },
            ]
            }
        )
    );
}
