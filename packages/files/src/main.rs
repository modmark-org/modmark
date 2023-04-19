use base64::{engine::general_purpose, Engine as _};
use serde_json::{json, Value};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

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

fn manifest() {
    print!(
        "{}",
        serde_json::to_string(&json!(
            {
            "name": "files",
            "version": "0.1",
            "description": "This package provides file access.",
            "transforms": [
                {
                    "from": "textfile",
                    "to": ["html", "latex"],
                    "arguments": [],
                },
                {
                    "from": "image",
                    "to": ["html", "latex"],
                    "arguments": [
                        {"name": "alt", "default": "", "description": "Alternative text for the image"},
                        {
                            "name": "caption",
                            "default": "",
                            "description": "The caption for the image."
                        },
                        {
                            "name": "label",
                            "default": "",
                            "description": "The label to use for the image, to be able to refer to it from the document."
                        },
                        {
                            "name": "width",
                            "default": 1.0,
                            "type": "f64",
                            "description":
                                "\
                                The width of the image resulting image. \
                                For LaTeX this is ratio to the document's text area width. \
                                For HTML this is ratio to the width of the surrounding figure tag (created automatically).\
                                "
                        },
                        {
                            "name": "embed",
                            "default": "false",
                            "type": ["true", "false"],
                            "description": "Decides if the provided image should be embedded in the HTML document."
                        },
                    ],
                },
                {
                    "from": "include",
                    "to": ["any"],
                    "arguments": [],
                    "unknown-content": true
                }
            ]
            }
        ))
        .unwrap()
    );
}

fn transform(from: &str, to: &str) {
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };
    match from {
        "textfile" => transform_text(input, to),
        "image" => transform_image(input, to),
        "include" => transform_include(input),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_text(input: Value, to: &str) {
    match to {
        "html" | "latex" => {
            let path = input["data"].as_str().unwrap().trim();
            if let Ok(contents) = fs::read_to_string(path) {
                let text = json!([{"name": "__text", "data": contents}]);
                let json = json!({"name": "__paragraph", "arguments": {}, "children": text}).to_string();
                print!("[{json}]");
            } else {
                eprintln!("File could not be accessed at {path}");
            }
        }
        other => {
            eprintln!("Cannot convert file to {other}");
        }
    }
}

fn transform_image(input: Value, to: &str) {
    match to {
        "html" => {
            let path = input["data"].as_str().unwrap().trim();
            let alt = {
                let alt_text = input["arguments"]["alt"].as_str().unwrap();
                if alt_text.is_empty() {
                    eprintln!("Missing alt text");
                    "Missing alt text".to_string()
                } else {
                    alt_text.replace('"', "&quot;")
                }
            };
            let width = input["arguments"]["width"].as_f64().unwrap().clamp(0.0, f64::MAX);
            let caption = input["arguments"]["caption"].as_str().unwrap();
            let label = input["arguments"]["label"].as_str().unwrap();
            let embed = input["arguments"]["embed"].as_str().unwrap();

            let percentage = (width * 100.0).round() as i32;
            let style = format!("style=\"width:{percentage}%\"");
            let id = if label.is_empty() {
                String::new()
            } else {
                format!("id=\"{label}\"")
            };

            let img_src = if embed == "false" {
                String::from(path)
            } else {
                let read_res = fs::read(path);
                let ext_opt = Path::new(path).extension().and_then(OsStr::to_str);
                if let Ok(contents) = read_res {
                    let encoded: String = general_purpose::STANDARD_NO_PAD.encode(contents);
                    if let Some(ext) = ext_opt {
                        match ext {
                            "svg" => format!("data:image/svg+xml;base64,{encoded}"),
                            "jpg" | "jpeg" | "png" => format!("data:image/png;base64,{encoded}"),
                            _ => {
                                eprintln!("Unexpected file extension.");
                                format!("data:image/png;base64,{encoded}")
                            }
                        }
                    } else {
                        eprintln!("File type could not be inferred from path.");
                        format!("data:image/png;base64,{encoded}")
                    }
                } else {
                    eprintln!("File could not be accessed at {path}.");
                    return;
                }
            };
            let img_str = format!("<img src=\"{img_src}\" {id} {style} alt=\"");

            let mut v = vec![];
            v.push(raw!("<figure>\n"));
            v.push(raw!(img_str));
            v.push(json!({"name": "__text", "data": alt}));
            v.push(raw!("\"/>\n"));
            if !caption.is_empty() {
                v.push(raw!("<figcaption>"));
                v.push(json!({"name": "inline_content", "data": caption}));
                v.push(raw!("</figcaption>\n"));
            }
            v.push(raw!("</figure>\n"));

            print!("{}", json!(v));
        }
        "latex" => {
            let path = input["data"].as_str().unwrap().trim();
            let width = input["arguments"]["width"].as_f64().unwrap().clamp(0.0, f64::MAX);
            let caption = input["arguments"]["caption"].as_str().unwrap();
            let label = input["arguments"]["label"].as_str().unwrap();

            let img_str = {
                if let Some(ext) = Path::new(path).extension().and_then(OsStr::to_str) {
                    match ext {
                        "svg" => format!("\\includesvg[width={width}\\textwidth]{{{path}}}\n"),
                        "png" | "jpg" | "jpeg" => {
                            format!("\\includegraphics[width={width}\\textwidth]{{{path}}}\n")
                        },
                        _ => {
                            eprintln!("Unexpected file extension.");
                            format!("\\includegraphics[width={width}\\textwidth]{{{path}}}\n")
                        }
                    }
                } else {
                    eprintln!("File type could not be inferred from the provided path.");
                    format!("\\includegraphics[width={width}\\textwidth]{{{path}}}\n")
                }
            };

            let mut v = vec![];

            v.push(raw!("\\begin{figure}[H]\n"));
            v.push(raw!("\\centering\n"));
            v.push(raw!(img_str));
            if !caption.is_empty() {
                v.push(raw!("\\caption{"));
                v.push(json!({"name": "inline_content", "data": caption}));
                v.push(raw!("}\n"));
            }
            if !label.is_empty() {
                v.push(raw!("\\label{"));
                v.push(json!({"name": "inline_content", "data": label}));
                v.push(raw!("}\n"))
            }
            v.push(raw!("\\end{figure}\n"));

            print!("{}", json!(v));
        }
        other => {
            eprintln!("Cannot convert file to {other}");
        }
    }
}

// Because everything inside is reparsed, we do not match against output format
fn transform_include(input: Value) {
    let path = input["data"].as_str().unwrap().trim();
    if let Ok(contents) = fs::read_to_string(path) {
        let json = json!({"name": "block_content", "data": contents}).to_string();
        print!("[{json}]");
    } else {
        eprintln!("File could not be accessed at {path}");
    }
}
