use std::io::Read;
use std::{env, io};

use latex2mathml::{latex_to_mathml, DisplayStyle};
use serde_json::{json, Value};

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
            eprintln!("Invalid action {other}");
        }
    }
}

fn manifest() {
    print!(
        "{}",
        serde_json::to_string(&json!(
            {
            "name": "math",
            "version": "0.1",
            "description": "This package provides inline and multiline [math] modules",
            "transforms": [
                {
                    "from": "math",
                    "to": ["html", "latex"],
                    "arguments": [
                        {
                            "name": "import",
                            "default": "false",
                            "type": ["true", "false"],
                            "description": r#"If set to "true", a Mathjax import will be added to HTML outputs for maximum compatibility"#
                        }
                    ],
                }
            ]
            }
        ))
        .unwrap()
    );
}

fn transform(from: &str, to: &str) {
    match from {
        "math" => transform_math(to),
        other => {
            eprintln!("Package does not support transforming from {other}");
        }
    }
}

fn transform_math(to: &str) {
    let json: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };

    match to {
        "html" => math_to_html(&json),
        "latex" => math_to_latex(&json),
        other => {
            eprintln!("Package does not support transforming math to {other}");
        }
    }
}

fn math_to_html(json: &Value) {
    // This import is added as part of the output, but when we get variables up and running, we
    // would stick it into the import variable
    let import = r#"<script id="MathJax-script" async src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/mml-chtml.js"></script>"#;

    let import_opt = json["arguments"]["import"]
        .as_str()
        .expect(r#""import" as string"#);
    let do_import = !import_opt.eq_ignore_ascii_case("false");
    if !import_opt.eq_ignore_ascii_case("true") && !import_opt.eq_ignore_ascii_case("false") {
        eprintln!(
            r#"Argument "import" expected to be "true" or "false", but was actually "{import_opt}""#
        );
    }

    let body = json["data"].as_str().expect("Data as string");
    let inline = json["inline"].as_bool().expect("Inline as bool");
    let inline_hint = if inline {
        DisplayStyle::Inline
    } else {
        DisplayStyle::Block
    };
    let result = latex_to_mathml(body, inline_hint);
    match result {
        Ok(mathml) => {
            if do_import {
                println!(
                    "{}",
                    json! {[
                        raw!(import),
                        raw!(mathml)
                    ]}
                );
            } else {
                println!(
                    "{}",
                    json! {[
                        raw!(mathml)
                    ]}
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to parse latex: {e}");
        }
    }
}

fn math_to_latex(json: &Value) {
    let body = json["data"].as_str().expect("Data as string");
    if json["inline"].as_bool().expect("Inline as bool") {
        println!(
            "{}",
            json! {[
                raw!(format!("${}$", body.replace('$', r"\$")))
            ]}
        );
    } else {
        println!(
            "{}",
            json! {[
                raw!(r"\begin{equation}"),
                raw!(body),
                raw!(r"\end{equation}")
            ]}
        );
    }
}
