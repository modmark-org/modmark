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
        "test" => test(),
        "manifest" => manifest(),
        "transform" => transform(&args[1], &args[2]),
        other => {
            eprintln!("Invalid action {other}")
        }
    }
}

fn test() {
    let lang = &"py".to_string();
    let code = "print(\"hello\")\na=5";
    let tm = &"github".to_string();
    let indent = &"4".to_string();
    let html = get_highlighted(code, indent, lang, tm);
    let json = json!({"name": "raw", "data": html}).to_string();
    println!("{}", json);
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
                    {"name": "indent", "default": "4", "description": "The size indents will be adjusted to (from the default 4)"},
                    {"name": "fontsize", "default": "12", "description": "The size of the font"},
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
    match to.as_str() {
        "html" => {
            let input: Value = {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();
                serde_json::from_str(&buffer).unwrap()
            };

            let code = input["data"].as_str().unwrap();
            let Value::String(lang) = &input["arguments"]["lang"] else {
                panic!("No lang argument was provided");
            };
            let Value::String(indent) = &input["arguments"]["indent"] else {
                panic!("No indent argument was provided");
            };
            let Value::String(size) = &input["arguments"]["fontsize"] else {
                panic!("No fontsize argument was provided");
            };
            let Value::String(tm) = &input["arguments"]["theme"] else {
                panic!("No theme argument was provided");
            };
            let Value::String(bg) = &input["arguments"]["bg"] else {
                panic!("No bg argument was provided");
            };

            let (highlighted, default_bg) = get_highlighted(code, indent, lang, tm);
            let style = get_style(size, bg, default_bg);

            let mut html = String::new();
            write!(html, "<pre {style}>");
            write!(html, "{}{}", highlighted, "</pre>");

            let json = json!({"name": "raw", "data": html}).to_string();
            let mut output = String::new();
            write!(output, "{}", json);
            print!("[{output}]");
        }
        other => {
            eprintln!("Cannot convert code to {other}");
            return;
        }
    }
}

fn get_highlighted(code: &str, indent: &String, lang: &String, tm: &String) -> (String, Option<Color>) {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ss.find_syntax_by_extension(lang).unwrap();
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
    let indent_size = indent.parse::<usize>().unwrap_or(4);
    // avoiding lines() here because we want to include the final newline
    for line in code.split("\n").map(|s| s.trim_end_matches("\r")) {
        let len = line.len();
        let line = line.trim_start_matches(" ");
        let indents = (len - line.len()) / 4; // assuming 4 is default tab
        let new_indent = indents * indent_size;
        let line = format!("{}{}", " ".repeat(new_indent), line);
        let regions = h.highlight_line(line.as_str(), &ss).unwrap();
        html.push(styled_line_to_highlighted_html(&regions[..], incl_bg).unwrap())
    }
    return (html.join("<br>"), theme.settings.background);
}

fn write_css_color(s: &mut String, c: Color) {
    if c.a != 0xFF {
        write!(s, "#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a).unwrap();
    } else {
        write!(s, "#{:02x}{:02x}{:02x}", c.r, c.g, c.b).unwrap();
    }
}

fn get_style(size: &String, bg: &String, default_bg: Option<Color>) -> String {
    let mut style = String::from("style=\"");
    write!(style, "font-size: {}px; background-color: ", size);

    if bg == "default" && default_bg.is_some() {
        write_css_color(&mut style, default_bg.unwrap());
    } else {
        write!(style, "#{}", bg);
    }

    style.push_str("\"");
    style
}
