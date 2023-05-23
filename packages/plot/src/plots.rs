use plotters::prelude::*;
use std::ops::Range;

use crate::eval::*;
use crate::utils::*;

type Points = Vec<(f64, f64)>;
type Plot = Vec<Points>;

fn get_function_plots(ctx: &mut PlotContext) -> Result<Vec<Plot>, String> {
    let mut plots = Vec::new();

    for function in ctx.data.split('\n') {
        if function.trim().is_empty() {
            continue;
        }
        let mut x_iter = ctx.rm.get_point_range('x')?;

        // Add extra x_values at the function's asymptotes if there are any, and store in a Vec
        let mut x_values = Vec::new();
        while let Some(x) = x_iter.next() {
            x_values.push(x);
            if let Some(nx) = x_iter.peek() {
                // We don't know exactly where ax is in relation to the asymptote, therefore we
                // have to add one on each side of the asymptote
                // (likely works without adding ax itself)
                if let Some(ax) = find_asymptote(&mut ctx.fn_ctx, function, (x, nx)) {
                    x_values.push(ax - PLOT_EPSILON);
                    x_values.push(ax);
                    x_values.push(ax + PLOT_EPSILON);
                }
            }
        }

        // Evaluate function at all x-values, clamp so that evaluations such as 1/0 are usable
        let points = x_values
            .iter()
            .map(|&x| {
                let eval = eval_function(&mut ctx.fn_ctx, function, &[x]).unwrap();
                let y = clamp_eval(eval);
                (x, y)
            })
            .collect::<Points>();

        let mut plot = Plot::new();
        let mut plot_idx = 0;
        plot.push(Points::new());

        for i in 0..points.len() {
            let &(x, y) = points.get(i).unwrap();

            // If current point is NaN, do not add it and start a new series of points
            if y.is_nan() {
                plot.push(Points::new());
                plot_idx += 1;
                continue;
            }

            // If the current point is inside the function, we can always add it without extra steps
            let margin = ctx.rm.y_margin(y);
            if margin == 0.0 {
                plot[plot_idx].push((x, y));
                continue;
            }

            // If the current point is outside and the previous point is inside, we have to
            // approximate where the plot crosses the y-boundary. We then add this point and start
            // a new series of points (so that it doesn't draw a line to where it goes inside the
            // graph again).
            if i > 0 {
                if let Some(&(px, py)) = points.get(i - 1) {
                    if ctx.rm.y_margin(py) == 0.0 {
                        let point = get_point_at_boundary((x, y), (px, py), margin);
                        plot[plot_idx].push(point);

                        plot.push(Points::new());
                        plot_idx += 1;
                    }
                }
            }

            // If the current point is outside and the next point is inside, we once again have to
            // approximate where the plot crosses the y-boundary. Since a new series of points
            // was already started when we went outside the graph, we don't need to do that again.
            if let Some(&(nx, ny)) = points.get(i + 1) {
                if ctx.rm.y_margin(ny) == 0.0 {
                    let point = get_point_at_boundary((x, y), (nx, ny), margin);
                    plot[plot_idx].push(point);
                }
            }
        }

        plots.push(plot);
    }
    Ok(plots)
}

pub fn get_function_svg(ctx: &mut PlotContext) -> Result<String, String> {
    verify_functions(ctx, 1)?;
    let plots = get_function_plots(ctx)?;
    let mut buf = String::new();
    // Scope to drop root and chart before buf is needed again
    {
        let root = SVGBackend::with_string(&mut buf, (600, 600)).into_drawing_area();
        let x_range = Range::try_from(ctx.rm.get_user_range('x')?)?;
        let y_range = Range::try_from(ctx.rm.get_user_range('y')?)?;

        // Used to ensure that large numbers on y-axis are not cut off
        let (y_fr, y_to) = ctx.rm.get_user_range_endpoints('y')?;
        let digits = y_fr.abs().max(y_to.abs()).log10();

        let mut chart = ChartBuilder::on(&root)
            .margin(10)
            .x_label_area_size(10)
            .y_label_area_size(15.0 + 5.0 * digits)
            .build_cartesian_2d(x_range, y_range)
            .map_err(|_| String::from("Failed to build coordinate system."))?;

        chart
            .configure_mesh()
            .max_light_lines(0)
            .draw()
            .map_err(|_| String::from("Failed to draw coordinate system."))?;

        for plot in plots {
            let style = ctx.get_style();
            for points in plot {
                chart
                    .draw_series(LineSeries::new(points, style))
                    .map_err(|_| String::from("Failed to plot the function."))?;
            }
        }

        root.present()
            .map_err(|_| String::from("Failed to create SVG."))?;
    }
    Ok(buf)
}

fn get_list_points(ctx: &mut PlotContext) -> Result<Points, String> {
    let mut points = Vec::new();
    for line in ctx.data.split('\n') {
        let values = line
            .split(' ')
            .filter_map(|v| v.parse::<f64>().ok())
            .collect::<Vec<f64>>();
        if values.len() != 2 {
            return Err(String::from("Invalid data in list of values."));
        } else {
            points.push((values[0], values[1]))
        }
    }
    Ok(points)
}

pub fn get_list_svg(ctx: &mut PlotContext) -> Result<String, String> {
    let points = get_list_points(ctx)?;

    let mut buf = String::new();
    // Scope to drop root and chart before buf is needed again
    {
        let root = SVGBackend::with_string(&mut buf, (600, 600)).into_drawing_area();
        let x_range = Range::try_from(ctx.rm.get_user_range('x')?)?;
        let y_range = Range::try_from(ctx.rm.get_user_range('y')?)?;

        // Used to ensure that large numbers on y-axis are not cut off
        let (y_fr, y_to) = ctx.rm.get_user_range_endpoints('y')?;
        let digits = y_fr.abs().max(y_to.abs()).log10();

        let mut chart = ChartBuilder::on(&root)
            .margin(10)
            .x_label_area_size(10)
            .y_label_area_size(15.0 + 5.0 * digits)
            .build_cartesian_2d(x_range, y_range)
            .map_err(|_| String::from("Failed to build coordinate system."))?;

        chart
            .configure_mesh()
            .max_light_lines(0)
            .draw()
            .map_err(|_| String::from("Failed to draw coordinate system."))?;

        let style = ctx.get_style();

        if let Some(size) = ctx.point_size.map(|size| size as u32) {
            chart
                .draw_series(
                    points
                        .iter()
                        .map(|(x, y)| Circle::new((*x, *y), size, style)),
                )
                .map_err(|_| String::from("Failed to plot the points."))?;
        }

        if let Some(true) = ctx.discrete {
            chart
                .draw_series(LineSeries::new(points.into_iter(), style))
                .map_err(|_| String::from("Failed to connect the points."))?;
        }

        root.present()
            .map_err(|_| String::from("Failed to create SVG."))?;
    }

    Ok(buf)
}
