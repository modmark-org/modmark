use base64::{engine::general_purpose, Engine as _};
use plotters::prelude::*;
use serde_json::{json, Value};
use std::fs::File;
use std::io::Write;

use plot::{get_shape_style, raw};

fn get_svg_from_list(
    data: &str,
    x_range: &str,
    y_range: &str,
    shape_style: ShapeStyle,
    connect: bool,
    point_size: u32,
) -> Result<String, String> {
    // string where svg is stored
    let mut buf = String::new();

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

    let mut points = Vec::new();
    for line in data.split('\n') {
        let values = line
            .split(' ')
            .filter_map(|v| v.parse::<f32>().ok())
            .collect::<Vec<f32>>();
        if values.len() != 2 {
            return Err("Invalid data in list of values.".to_string());
        } else {
            points.push((values[0], values[1]))
        }
    }

    {
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
            .draw_series(
                points
                    .iter()
                    .map(|(x, y)| Circle::new((*x, *y), point_size, shape_style)),
            )
            .map_err(|_| "Failed to connect the points.".to_string())?;

        if connect {
            chart
                .draw_series(LineSeries::new(points.into_iter(), shape_style))
                .map_err(|_| "Failed to connect the points.".to_string())?;
        }

        root.present()
            .map_err(|_| "Failed to create SVG.".to_string())?;
    }

    Ok(buf)
}

pub fn transform_plot_list(input: Value) {
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

    let line_color = input["arguments"]["color"].as_str().unwrap();
    let line_width = input["arguments"]["line_width"].as_u64().unwrap() as u32;
    let shape_style = get_shape_style(line_color, line_width);
    let connect = input["arguments"]["connect"].as_str().unwrap() == "true";
    let point_size = input["arguments"]["point_size"].as_u64().unwrap() as u32;

    let svg = match get_svg_from_list(function, x_range, y_range, shape_style, connect, point_size)
    {
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
            write!(file, "{svg}").unwrap();
        } else {
            eprintln!("Could not open the specified file.");
        }
    }
}
