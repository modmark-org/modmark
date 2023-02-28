use std::env;
use std::fmt::Write;
use std::io::{self, Read};

use serde_json::{json, Value};
<<<<<<< HEAD
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Color};
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
=======
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{Color, ThemeSet};
use syntect::html::highlighted_html_for_string;
>>>>>>> ae9220c (GH-60: Add minimal version of code package)

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
<<<<<<< HEAD
        "name": "code",
=======
        "name": "Standard code package",
>>>>>>> ae9220c (GH-60: Add minimal version of code package)
        "version": "0.1",
        "description": "This package provides syntax highlighting in [code] modules",
        "transforms": [
            {
                "from": "code",
                "to": ["html"],
                "arguments": [
<<<<<<< HEAD
                    {"name": "lang", "default": "txt", "description":
                        "The language to be highlighted. For available languages, see \
                        https://github.com/sublimehq/Packages"},
                    {"name": "font_size", "default": "12", "description": "The size of the font"},
                    {"name": "tab_size", "default": "4", "description": "The size tabs will be adjusted to"},
                    {"name": "theme", "default": "mocha", "description":
                        "Theme of the code section. For available themes, see \
                        https://docs.rs/syntect/latest/syntect/highlighting/struct.ThemeSet.html#method.load_defaults"},
                    {"name": "bg", "default": "default", "description": "Background of the code section"},
=======
                    {"name": "lang", "description": "The language to be highlighted"},
                    {"name": "fontsize", "default": "12", "description": "The size of the font"}
>>>>>>> ae9220c (GH-60: Add minimal version of code package)
                ],
            }
        ]
        }
    ))
    .unwrap());
}

<<<<<<< HEAD
fn transform(from: &str, to: &str) {
    match from {
        "code" => transform_code(to),
        other => {
            eprintln!("Package does not support {other}");
=======
fn transform(from: &String, to: &String) {
    match from.as_str() {
        "code" => transform_code(to),
        other => {
            eprintln!("Package does not support {other}");
            return;
>>>>>>> ae9220c (GH-60: Add minimal version of code package)
        }
    }
}

<<<<<<< HEAD
fn transform_code(to: &str) {
    macro_rules! get_arg {
        ($input:expr, $arg:expr) => {
            if let Value::String(val) = &$input["arguments"][$arg] {
                val
            } else {
                panic!("No {} argument was provided", $arg);
            }
        }
    }
    match to {
=======
fn transform_code(to: &String) {
    match to.as_str() {
>>>>>>> ae9220c (GH-60: Add minimal version of code package)
        "html" => {
            let input: Value = {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();
                serde_json::from_str(&buffer).unwrap()
            };

<<<<<<< HEAD
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
        }
    }
}

fn get_style(font_size: &String, tab_size: &String, bg: &String, default_bg: Option<Color>) -> String {
    let hex = if bg == "default" && default_bg.is_some() {
        let c = default_bg.unwrap();
        format!("{:02x}{:02x}{:02x}", c.r, c.g, c.b)
    } else {
        bg.clone()
    };

    let mut style = String::from("style=\"");
    write!(style, "box_sizing: border_box; ").unwrap();
    write!(style, "padding: 0.5rem; ").unwrap();
    write!(style, "tab-size: {tab_size}; ").unwrap();
    write!(style, "font-size: {font_size}px; ").unwrap();
    write!(style, "background-color: #{hex}; ").unwrap();
    style.push('\"');
    style
}

fn get_highlighted(code: &str, lang: &String, tm: &str) -> (String, Option<Color>) {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = match tm {
        "ocean_dark" => &ts.themes["base16-ocean.dark"],
        "ocean_light" => &ts.themes["base16-ocean.light"],
        "mocha" => &ts.themes["base16-mocha.dark"],
        "eighties" => &ts.themes["base16-eighties.dark"],
        "github" => &ts.themes["InspiredGitHub"],
        "solar_dark" => &ts.themes["Solarized (dark)"],
        "solar_light" => &ts.themes["Solarized (light)"],
        _ => &ts.themes["InspiredGitHub"]
    };

    if let Some(syntax) = ss.find_syntax_by_token(lang) {
        let mut h = HighlightLines::new(syntax, theme);
        let incl_bg = IncludeBackground::No;
        let mut html: Vec<String> = vec![];

        // avoiding lines() here because we want to include the final newline
        for line in code.split('\n').map(|s| s.trim_end_matches('\r')) {
            let regions = h.highlight_line(line, &ss).unwrap();
            html.push(styled_line_to_highlighted_html(&regions[..], incl_bg).unwrap())
        }
        (html.join("<br>"), theme.settings.background)
    } else {
        eprintln!("Invalid language: {lang}");
        let html = code
            .split('\n')
            .map(|s| s.trim_end_matches('\r'))
            .collect::<Vec<&str>>();
        (html.join("<br>"), Some(Color{r: 200, g: 200, b: 200, a: 255}))
    }
}
=======
            let Value::String(lang) = &input["arguments"]["lang"] else {
                panic!("No lang argument was provided");
            };
            let Value::String(size) = &input["arguments"]["fontsize"] else {
                panic!("No fontsize argument was provided");
            };

            let code = input["data"].as_str().unwrap();

            let style = format!("style=\\\"font-size:{size}px\\\"");
            let ss = SyntaxSet::load_defaults_newlines();
            let ts = ThemeSet::load_defaults();
            let syntax = ss.find_syntax_by_extension(lang).unwrap();
            let theme = &ts.themes["base16-ocean.light"];

            let html = highlighted_html_for_string(code, &ss, syntax, theme).unwrap();
            let formatted = html
                .replace("\"", "\\\"")
                .lines()
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>()
                .join("<br>");

            let mut output = String::new();
            output.push('[');
            write!(output, r#"{{"name": "raw", "data": "<code {style}>"}},"#).expect("");
            write!(output, r#"{{"name": "raw", "data": "{formatted}"}},"#).expect("");
            output.push_str(r#"{"name": "raw", "data": "</code>"}"#);
            output.push(']');

            print!("{output}");
        }
        other => {
            eprintln!("Cannot convert code to {other}");
            return;
        }
    }
}
>>>>>>> ae9220c (GH-60: Add minimal version of code package)
