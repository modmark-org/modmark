use base64::{engine::general_purpose, Engine as _};
use serde_json::{json, Value};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

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
                    "to": ["html", "latex"],
                    "arguments": [],
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
            match fs::read_to_string(path) {
                Ok(contents) => {
                    let data = if to == "html" {
                        format!("<p>{contents}</p>")
                    } else {
                        contents
                    };
                    let json = json!({"name": "raw", "data": data}).to_string();
                    print!("[{json}]");
                }
                _ => {
                    let json = json!({"name": "raw", "data": ""}).to_string();
                    print!("[{json}]");
                    eprintln!("File could not be accessed at {path}")
                }
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
            let width = input["arguments"]["width"].as_f64().unwrap();
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
            let img_str = format!("<img src=\"{img_src}\" {id} {style}/>\n");

            let mut v = vec![];
            v.push(json!({"name": "raw", "data": "<figure>\n"}));
            v.push(json!({"name": "raw", "data": img_str}));
            if !caption.is_empty() {
                v.push(json!({"name": "raw", "data": "<figcaption>"}));
                v.push(json!({"name": "inline_content", "data": caption}));
                v.push(json!({"name": "raw", "data": "</figcaption>\n"}));
            }
            v.push(json!({"name": "raw", "data": "</figure>\n"}));

            print!("{}", json!(v));
        }
        "latex" => {
            let path = input["data"].as_str().unwrap().trim();
            let width = input["arguments"]["width"].as_f64().unwrap();
            let caption = input["arguments"]["caption"].as_str().unwrap();
            let label = input["arguments"]["label"].as_str().unwrap();

            let img_str = {
                if let Some(ext) = Path::new(path).extension().and_then(OsStr::to_str) {
                    match ext {
                        "svg" => format!("\\includesvg[width={width}\\textwidth]{path}\n"),
                        "png" | "jpg" | "jpeg" => {
                            format!("\\includegraphics[width={width}\\textwidth]{path}\n")
                        },
                        _ => {
                            eprintln!("Unexpected file extension.");
                            format!("\\includegraphics[width={width}\\textwidth]{path}\n")
                        }
                    }
                } else {
                    eprintln!("File type could not be inferred from the provided path.");
                    format!("\\includegraphics[width={width}\\textwidth]{path}\n")
                }
            };

            let mut v = vec![];

            v.push(String::from("\\begin{figure}[H]"));
            v.push(String::from("\\centering"));
            v.push(img_str);
            if !caption.is_empty() {
                v.push(format!("\\caption{}{}{}", "{", caption, "}"));
            }
            if !label.is_empty() {
                v.push(format!("\\label{}{}{}", "{", label, "}"));
            }
            v.push(String::from("\\end{figure}"));

            let json = json!({"name": "raw", "data": v.join("\n")}).to_string();
            print!("[{json}]");
        }
        other => {
            eprintln!("Cannot convert file to {other}");
        }
    }
}

// Because everything inside is reparsed, we do not match against output format
fn transform_include(input: Value) {
    let path = input["data"].as_str().unwrap().trim();
    match fs::read_to_string(path) {
        Ok(contents) => {
            let json = json!({"name": "block_content", "data": contents}).to_string();
            print!("[{json}]");
        }
        _ => {
            let json = json!({"name": "raw", "data": ""}).to_string();
            print!("[{json}]");
            eprintln!("File could not be accessed at {path}")
        }
    }
}
