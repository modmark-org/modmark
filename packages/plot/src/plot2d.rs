use base64::{engine::general_purpose, Engine as _};
use plotters::prelude::*;
use serde_json::{json, Value};
use std::fs::File;
use std::io::Write;

use plot::{eval_function, get_calc_funcs, get_shape_style, raw, verify_function};

fn get_svg_2d(
    function: &str,
    x_range: &str,
    y_range: &str,
    shape_style: ShapeStyle,
) -> Result<String, String> {
    // verify that function is valid
    if !verify_function(function, 1) {
        return Err("Invalid function.".to_string());
    }

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

        // quotient to scale range of points and mean of x_fr, x_to
        let q = (x_to - x_fr) / 200.0;
        let m = (x_fr + x_to) / 2.0;
        let calc_funcs = get_calc_funcs();

        // create a range of 400 points (same as svg width), scale the points to x_range and
        // filter out undefined values
        let points = (-100..=100)
            .map(|x| x as f32 * q + m)
            .map(|x| (x, eval_function(function, vec![x], &calc_funcs)))
            .filter_map(|(x, y)| {
                if let Ok(v) = y {
                    if v != f64::INFINITY && v != f64::NEG_INFINITY {
                        Some((x, v as f32))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<(f32, f32)>>();

        // filter out all points where both neighbours are out of bounds
        // and split into different series, based on where plot goes outside bounds
        let mut series: Vec<Vec<(f32, f32)>> = vec![vec![]];
        let (_, y) = points[1];
        // special case for first point
        if y >= y_fr && y <= y_to {
            let i = series.len() - 1;
            series.get_mut(i).unwrap().push(points[0]);
        }
        for i in 1..points.len() - 1 {
            let (_, ly) = points[i - 1];
            let (_, ry) = points[i + 1];
            if (ly >= y_fr && ly <= y_to) || (ry >= y_fr && ry <= y_to) {
                let j = series.len() - 1;
                series.get_mut(j).unwrap().push(points[i]);
            } else {
                series.push(vec![]);
            }
        }
        // special case for last point
        let (_, y) = points[points.len() - 2];
        if y >= y_fr && y <= y_to {
            let i = series.len() - 1;
            series.get_mut(i).unwrap().push(points[points.len() - 1]);
        }

        let root = SVGBackend::with_string(&mut buf, (400, 400)).into_drawing_area();
        let mut chart = ChartBuilder::on(&root)
            .margin(10)
            .set_left_and_bottom_label_area_size(20)
            .build_cartesian_2d(x_fr..x_to, y_fr..y_to)
            .map_err(|_| "Failed to build coordinate system.".to_string())?;
        chart
            .configure_mesh()
            .max_light_lines(0)
            .draw()
            .map_err(|_| "Failed to draw coordinate system.".to_string())?;
        for s in series {
            chart
                .draw_series(LineSeries::new(s, shape_style))
                .map_err(|_| "Failed to plot the function.".to_string())?
                .legend(|(x, y)| PathElement::new(vec![(x, y), (x, y)], RED));
        }
        root.present()
            .map_err(|_| "Failed to create SVG.".to_string())?;
    }
    Ok(buf)
}

pub fn transform_plot_2d(input: Value) {
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

    let line_color = input["arguments"]["line_color"].as_str().unwrap();
    let line_width = input["arguments"]["line_width"].as_u64().unwrap() as u32;
    let shape_style = get_shape_style(line_color, line_width);
    let svg = match get_svg_2d(function, x_range, y_range, shape_style) {
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
