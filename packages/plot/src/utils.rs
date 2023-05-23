use crate::eval::new_function_context;
use base64::{engine::general_purpose, Engine as _};
use evalexpr::HashMapContext;
use plotters::prelude::*;
use plotters::style::ShapeStyle;
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::ops::Range;

pub struct PlotContext {
    pub data: String,
    pub rm: RangeManager,
    pub fn_ctx: HashMapContext,
    pub fn_idx: usize,
    pub line_width: u64,
    pub colors: Vec<RGBColor>,
    pub discrete: Option<bool>,
    pub point_size: Option<u64>,
    pub svg_info: SVGInfo,
}

impl PlotContext {
    pub fn new(input: Value) -> Result<Self, String> {
        let data = input["data"].as_str().unwrap().to_string();
        let x_fr = input["arguments"]["x_from"].as_f64().unwrap();
        let x_to = input["arguments"]["x_to"].as_f64().unwrap();
        let y_fr = input["arguments"]["y_from"].as_f64().unwrap();
        let y_to = input["arguments"]["y_to"].as_f64().unwrap();
        let samples = input["arguments"]["samples"].as_u64().unwrap_or(0);

        let line_width = input["arguments"]["line_width"].as_u64().unwrap();
        let color_arg = match input["arguments"]["color"].as_str() {
            Some(color_arg) => color_arg,
            None => input["arguments"]["colors"].as_str().unwrap(),
        };

        let discrete = input["arguments"]["discrete"].as_str().map(|s| s == "true");
        let point_size = input["arguments"]["point_size"].as_u64();

        let width = input["arguments"]["width"]
            .as_f64()
            .unwrap()
            .clamp(0.0, f64::MAX);
        let label = input["arguments"]["label"].as_str().unwrap().to_string();
        let caption = input["arguments"]["caption"].as_str().unwrap().to_string();
        let save = input["arguments"]["save"].as_str().unwrap().to_string();

        let rm = RangeManager::new(samples, x_fr, x_to, y_fr, y_to);
        let fn_ctx = new_function_context();
        let fn_idx = 0;

        let mut colors = Vec::new();
        for hex in color_arg.split(',').map(|s| s.trim()) {
            let color = match hex.strip_prefix('#') {
                Some(color) => color,
                None => return Err(String::from("Each hex code should start with a hashtag.")),
            };
            if color.len() != 6 {
                return Err(String::from("Invalid length of hex code."));
            }
            colors.push(RGBColor(
                u8::from_str_radix(&color[0..2], 16).map_err(|e| e.to_string())?,
                u8::from_str_radix(&color[2..4], 16).map_err(|e| e.to_string())?,
                u8::from_str_radix(&color[4..6], 16).map_err(|e| e.to_string())?,
            ));
        }

        let svg_info = SVGInfo {
            width,
            label,
            caption,
            save,
        };

        Ok(Self {
            rm,
            fn_ctx,
            fn_idx,
            data,
            line_width,
            colors,
            discrete,
            point_size,
            svg_info,
        })
    }

    pub fn get_style(&mut self) -> ShapeStyle {
        let style = ShapeStyle {
            color: self.colors[self.fn_idx].into(),
            filled: true,
            stroke_width: self.line_width as u32,
        };
        self.fn_idx = (self.fn_idx + 1) % self.colors.len();
        style
    }
}

pub struct SVGInfo {
    width: f64,
    label: String,
    caption: String,
    save: String,
}

// Iterator for floats with custom steps, because Range<f64> only supports integer steps
pub struct FloatIterator {
    from: f64,
    to: f64,
    step: f64,
    curr: f64,
}

impl Iterator for FloatIterator {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr <= self.to {
            let next = self.curr;
            self.curr += self.step;
            Some(next)
        } else {
            None
        }
    }
}

impl FloatIterator {
    pub fn new(from: f64, to: f64, step: f64) -> Result<Self, String> {
        if from > to {
            Err(String::from(
                "Can not construct iterator with start point greater than end point.",
            ))
        } else if step <= 0.0 {
            Err(String::from(
                "Can not construct iterator without strictly positive step.",
            ))
        } else if !from.is_finite() || !to.is_finite() || !step.is_finite() {
            Err(String::from(
                "Can not construct iterator without finite parameters.",
            ))
        } else {
            Ok(Self {
                from,
                to,
                step,
                curr: from,
            })
        }
    }

    pub fn peek(&self) -> Option<f64> {
        if self.curr <= self.to {
            Some(self.curr)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        ((self.to - self.from) / self.step) as usize
    }
}

// Used for build_cartesian_2d
impl TryFrom<FloatIterator> for Range<f64> {
    type Error = String;

    fn try_from(value: FloatIterator) -> Result<Self, Self::Error> {
        if value.step != 1.0 {
            Err(String::from(
                "Can not convert to Range<f64> when step is not 1.0.",
            ))
        } else {
            Ok(value.from..value.to)
        }
    }
}

// Should be relatively easy to extend this to three axes if needed
pub struct RangeManager {
    user_range_x: (f64, f64),
    user_range_y: (f64, f64),
    points: f64,
}

impl RangeManager {
    pub fn new(points: u64, x_fr: f64, x_to: f64, y_fr: f64, y_to: f64) -> Self {
        Self {
            user_range_x: (x_fr, x_to),
            user_range_y: (y_fr, y_to),
            points: points as f64,
        }
    }

    pub fn get_user_range_endpoints(&self, axis: char) -> Result<(f64, f64), String> {
        match axis {
            'x' => Ok((self.user_range_x.0, self.user_range_x.1)),
            'y' => Ok((self.user_range_y.0, self.user_range_y.1)),
            _ => Err(String::from("Invalid axis")),
        }
    }

    pub fn get_user_range(&self, axis: char) -> Result<FloatIterator, String> {
        let (fr, to) = self.get_user_range_endpoints(axis)?;
        FloatIterator::new(fr, to, 1.0)
    }

    pub fn get_point_range(&self, axis: char) -> Result<FloatIterator, String> {
        let (fr, to) = self.get_user_range_endpoints(axis)?;
        // Subtracting with f64 epsilon because otherwise floating point arithmetic may result in
        // last value being just above "to", example 4+1e^-16 is left out of (-4 to 4)
        let step = (to - fr) / self.points - f64::EPSILON;
        FloatIterator::new(fr, to, step)
    }

    // Returns 0 if y is inside the boundaries, otherwise the distance to the closest boundary
    pub fn y_margin(&self, y: f64) -> f64 {
        let (min, max) = self.user_range_y;
        if y > max {
            y - max
        } else if y < min {
            y - min
        } else {
            0.0
        }
    }
}

pub(crate) fn print_svg_latex(svg: String, ctx: &PlotContext) {
    let SVGInfo {
        width,
        label,
        caption,
        save,
    } = &ctx.svg_info;

    let content_hash = {
        let mut x = DefaultHasher::new();
        svg.hash(&mut x);
        x.finish()
    };

    // TODO: we maybe we should use the import! macro in this crate too?
    let import_svg =
        json!({"name": "set-add", "arguments": {"name": "imports"}, "data": "\\usepackage{svg}"});
    let import_floats =
        json!({"name": "set-add", "arguments": {"name": "imports"}, "data": "\\usepackage{float}"});

    let mut result = vec![
        // Import the needed packages:
        import_svg,
        import_floats,
        // Embed the image:
        // (TODO: later we might want to add a embed argument for this)
        Value::String(format!(
            r"\begin{{filecontents}}[noheader]{{{content_hash}.svg}}"
        )),
        Value::String(format!("\n{svg}")),
        Value::String(format!(r"\end{{filecontents}}")),
        // Start the figure
        Value::String("\n\\begin{figure}[H]\n\\centering\n".into()),
        Value::String(format!(
            r"\includesvg[width={width}\textwidth, inkscapelatex=false]{{{content_hash}}}"
        )),
        Value::String("\n".into()),
    ];

    if !caption.is_empty() {
        result.push(Value::String(format!("\\caption{{{caption}}}\n")));
    }

    if !label.is_empty() {
        result.push(Value::String(format!("\\label{{{label}}}\n")));
    }

    result.push(Value::String("\\end{figure}".into()));

    handle_save(&svg, save);

    println!("{}", &serde_json::to_string(&result).unwrap())
}

pub(crate) fn print_svg_html(svg: String, ctx: &PlotContext) {
    let encoded: String = general_purpose::STANDARD_NO_PAD.encode(&svg);
    let src = format!("data:image/svg+xml;base64,{encoded}");
    let percentage = (ctx.svg_info.width * 100.0).round() as i32;
    let style = format!("style=\"width:{percentage}%\"");
    let img_str = format!("<img src=\"{src}\" {style} ");

    let mut v = Vec::new();
    v.push(String::from("<figure>\n"));
    v.push(img_str);

    if !ctx.svg_info.label.is_empty() {
        v.push(String::from("id=\""));
        v.push(json!({"name": "__text", "data": ctx.svg_info.label}).to_string());
        v.push(String::from("\""));
    }
    v.push(String::from("/>\n"));
    if !ctx.svg_info.caption.is_empty() {
        v.push(String::from("<figcaption>"));
        v.push(json!({"name": "__text", "data": ctx.svg_info.caption}).to_string());
        v.push(String::from("</figcaption>\n"));
    }
    v.push(String::from("</figure>\n"));

    handle_save(&svg, &ctx.svg_info.save);

    print!("{}", json!(v));
}

fn handle_save(svg: &str, path: &str) {
    if !path.is_empty() {
        if let Ok(mut file) = File::create(path) {
            write!(file, "{svg}").unwrap();
        } else {
            eprintln!("Could not open the specified file.");
        }
    }
}

pub fn print_manifest() {
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
                    "to": ["html", "latex"],
                    "description": "Plot mathematical functions. Functions should be separated by newlines.",
                    "variables": {
                        "imports": {"type": "set", "access": "add"},
                    },
                    "arguments": [
                        {
                            "name": "x_from",
                            "description": "The left boundary of the x-axis.",
                            "type": "f64",
                            "default": -20.0,
                        },
                        {
                            "name": "x_to",
                            "description": "The right boundary of the x-axis.",
                            "type": "f64",
                            "default": 20.0,
                        },
                        {
                            "name": "y_from",
                            "description": "The lower boundary of the y-axis.",
                            "type": "f64",
                            "default": -20.0,
                        },
                        {
                            "name": "y_to",
                            "description": "The upper boundary of the y-axis.",
                            "type": "f64",
                            "default": 20.0,
                        },
                        {
                            "name": "samples",
                            "description": "The number of x-values for which the function is evaluated. If the plot appears jagged or squiggly, try reducing this value.",
                            "type": "uint",
                            "default": 400,
                        },
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
                            "name": "colors",
                            "description": "A list of colors that are used to plot the functions. They should be comma-separated and given in hexadecimal. Each hex code should be preceded by a hashtag.",
                            "default": "#0072BD,#D95319,#EDB120,#7E2F8E,#77AC30,#4DBEEE,#A2142F"
                        },
                    ],
                },
                {
                    "from": "plot-list",
                    "to": ["html", "latex"],
                    "description": "Plot a set of (x,y) values. The values should be placed on separate lines, and the x-y pair should space-separated.",
                    "variables": {
                        "imports": {"type": "set", "access": "add"},
                    },
                    "arguments": [
                        {
                            "name": "x_from",
                            "description": "The left bound of the x-axis.",
                            "type": "f64",
                            "default": -20.0,
                        },
                        {
                            "name": "x_to",
                            "description": "The right bound of the x-axis.",
                            "type": "f64",
                            "default": 20.0,
                        },
                        {
                            "name": "y_from",
                            "description": "The lower bound of the y-axis.",
                            "type": "f64",
                            "default": -20.0,
                        },
                        {
                            "name": "y_to",
                            "description": "The upper bound of the y-axis.",
                            "type": "f64",
                            "default": 20.0,
                        },
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
                            "description": "The color that is used in the plot, given in hexadecimal. The hex code should be preceded by a hashtag.",
                            "default": "#0072BD",
                        },
                        {
                            "name": "discrete",
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
