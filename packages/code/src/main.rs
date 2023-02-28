use std::env;
use std::fmt::Write;
use std::io::{self, Read};

use serde_json::{json, Value};
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Color};
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};

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
    print!("{}", serde_json::to_string(&json!(
        {
        "name": "Standard code package",
        "version": "0.1",
        "description": "This package provides syntax highlighting in [code] modules",
        "transforms": [
            {
                "from": "code",
                "to": ["html"],
                "arguments": [
                    {"name": "lang", "description": "The language to be highlighted"},
                    {"name": "font_size", "default": "12", "description": "The size of the font"},
                    {"name": "tab_size", "default": "4", "description": "The size tabs will be adjusted to"},
                    {"name": "theme", "default": "mocha", "description": "Theme of the code section"},
                    {"name": "bg", "default": "default", "description": "Background of the code section"},
                ],
            }
        ]
        }
    ))
    .unwrap());
}

fn transform(from: &String, to: &String) {
    match from.as_str() {
        "code" => transform_code(to),
        other => {
            eprintln!("Package does not support {other}");
            return;
        }
    }
}

fn transform_code(to: &String) {
    macro_rules! get_arg {
        ($input:expr, $arg:expr) => {
            if let Value::String(val) = &$input["arguments"][$arg] {
                val
            } else {
                panic!("No theme argument was provided");
            }
        }
    }
    match to.as_str() {
        "html" => {
            let input: Value = {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();
                serde_json::from_str(&buffer).unwrap()
            };

            let code = input["data"].as_str().unwrap();
            let lang = get_arg!(input, "lang");
            let font_size = get_arg!(input, "font_size");
            let tab_size = get_arg!(input, "tab_size");
            let theme = get_arg!(input, "theme");
            let bg = get_arg!(input, "bg");


            let (highlighted, default_bg) = get_highlighted(code, lang, theme);
            let style = get_style(font_size, tab_size, bg, default_bg);

            let html = format!(r#"<pre {style}>{highlighted}</pre>"#);
            let json = json!({"name": "raw", "data": html}).to_string();

            print!("[{json}]");
        }
        other => {
            eprintln!("Cannot convert code to {other}");
            return;
        }
    }
}

fn get_style(font_size: &String, tab_size: &String, bg: &String, default_bg: Option<Color>) -> String {
    let hex;
    if bg == "default" && default_bg.is_some() {
        let c = default_bg.unwrap();
        hex = format!("{:02x}{:02x}{:02x}", c.r, c.g, c.b);
    } else {
        hex = bg.clone();
    }

    let mut style = String::from("style=\"");
    write!(style, "box_sizing: border_box; ").unwrap();
    write!(style, "padding: 0.5rem; ").unwrap();
    write!(style, "tab-size: {}; ", tab_size).unwrap();
    write!(style, "font-size: {}px; ", font_size).unwrap();
    write!(style, "background-color: #{}; ", hex).unwrap();
    style.push('\"');
    style
}

fn get_highlighted(code: &str, lang: &String, tm: &String) -> (String, Option<Color>) {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = match ss.find_syntax_by_token(lang) {
        Some(sr) => sr,
        _ => ss.find_syntax_by_token("py").unwrap()
    };
    let theme = match tm.as_str() {
        "ocean_dark" => &ts.themes["base16-ocean.dark"],
        "ocean_light" => &ts.themes["base16-ocean.light"],
        "mocha" => &ts.themes["base16-mocha.dark"],
        "eighties" => &ts.themes["base16-eighties.dark"],
        "github" => &ts.themes["InspiredGitHub"],
        "solar_dark" => &ts.themes["Solarized (dark)"],
        "solar_light" => &ts.themes["Solarized (light)"],
        _ => &ts.themes["InspiredGitHub"]
    };
    let mut h = HighlightLines::new(syntax, theme);
    let incl_bg = IncludeBackground::No;
    let mut html: Vec<String> = vec![];

    // avoiding lines() here because we want to include the final newline
    for line in code.split("\n").map(|s| s.trim_end_matches("\r")) {
        let regions = h.highlight_line(line, &ss).unwrap();
        html.push(styled_line_to_highlighted_html(&regions[..], incl_bg).unwrap())
    }
    return (html.join("<br>"), theme.settings.background);
}
