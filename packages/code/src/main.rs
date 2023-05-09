use std::env;
use std::fmt::Write;
use std::io::{self, Read};

use serde_json::{json, Value};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Theme, ThemeSet};
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
use syntect::parsing::{SyntaxReference, SyntaxSet};

const VERBATIM_OVERRIDE_LATEX: &str = r"\makeatletter
\def\verbatim@nolig@list{\do\`\do\<\do\>\do\'\do\-}
\makeatother";

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
        "name": "code",
        "version": "0.1",
        "description": "This package provides syntax highlighting in [code] modules",
        "transforms": [
            {
                "from": "code",
                "to": ["html", "latex"],
                "arguments": [
                    {"name": "lang", "default": "txt", "description":
                        "The language to be highlighted. For available languages, see \
                        https://github.com/sublimehq/Packages"},
                    {"name": "font_size", "default": 12, "description": "The size of the font", "type": "uint"},
                    {"name": "tab_size", "default": 4, "description": "The size tabs will be adjusted to", "type": "uint"},
                    {"name": "theme", "default": "mocha", "description":
                        "Theme of the code section. For available themes, see \
                        https://docs.rs/syntect/latest/syntect/highlighting/struct.ThemeSet.html#method.load_defaults"},
                    {"name": "bg", "default": "default", "description": "Background of the code section"},
                ],
                "variables": {"imports": {"type": "set", "access": "add"}}
            }
        ]
        }
    ))
    .unwrap());
}

fn transform(from: &str, to: &str) {
    match from {
        "code" => transform_code(to),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_code(to: &str) {
    macro_rules! get_arg {
        ($input:expr, $arg:expr) => {
            if let Value::String(val) = &$input["arguments"][$arg] {
                val
            } else {
                panic!("No {} argument was provided", $arg);
            }
        };
    }
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };

    let code = input["data"].as_str().unwrap();
    let lang = get_arg!(input, "lang");
    let font_size = input["arguments"]["font_size"].as_u64().unwrap();
    let tab_size = input["arguments"]["tab_size"].as_u64().unwrap();
    let tm = get_arg!(input, "theme");
    let bg = get_arg!(input, "bg");

    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = match tm.as_str() {
        "ocean_dark" => &ts.themes["base16-ocean.dark"],
        "ocean_light" => &ts.themes["base16-ocean.light"],
        "mocha" => &ts.themes["base16-mocha.dark"],
        "eighties" => &ts.themes["base16-eighties.dark"],
        "github" => &ts.themes["InspiredGitHub"],
        "solar_dark" => &ts.themes["Solarized (dark)"],
        "solar_light" => &ts.themes["Solarized (light)"],
        _ => &ts.themes["InspiredGitHub"],
    };

    let syntax = ss.find_syntax_by_token(lang).unwrap_or_else(|| {
        eprintln!("Invalid language {lang}");
        ss.find_syntax_by_token("txt").unwrap()
    });

    match to {
        "html" => {
            let (highlighted, default_bg) = get_highlighted_html(code, theme, syntax, &ss);

            if let Value::Bool(inline) = &input["inline"] {
                let style = get_style_html(inline, font_size, tab_size, bg, default_bg);
                let html = if *inline {
                    format!(r#"<code {style}>{highlighted}</code>"#)
                } else {
                    format!(r#"<pre {style}>{highlighted}</pre>"#)
                };
                print!("[{}]", json!({"name": "raw", "data": html}));
            }
        }
        "latex" => {
            if let Value::Bool(inline) = &input["inline"] {
                if *inline {
                    print!("{}", latex_inline(code));
                } else {
                    if font_size != 12 {
                        eprintln!("Font size is not supported in LaTeX");
                    }
                    print!("{}", highlight_latex(code, tab_size, theme, syntax, &ss));
                }
            }
        }
        other => {
            eprintln!("Cannot convert code to {other}");
        }
    }
}

fn highlight_latex(
    code: &str,
    tab_size: u64,
    theme: &Theme,
    syntax: &SyntaxReference,
    ss: &SyntaxSet,
) -> String {
    macro_rules! import {
        ($e:expr) => {json!({"name": "set-add", "arguments": {"name": "imports"}, "data": $e})}
    }

    let mut h = HighlightLines::new(syntax, theme);
    let mut result: Vec<String> = vec![];
    let background_color = theme.settings.background.unwrap();
    let r = background_color.r;
    let g = background_color.g;
    let b = background_color.b;

    let code = code.replace('\t', &" ".repeat(tab_size.try_into().unwrap()));

    result.push(format!(
        "\\definecolor{{background}}{{RGB}}{{{r},{g},{b}}}\n"
    ));
    result.push("\\begin{tcolorbox}[colback=background, frame empty]\n".to_string());
    result.push("\\begin{Verbatim}[commandchars=\\\\\\{\\}]\n".to_string());

    for line in code.split('\n').map(|s| s.trim_end_matches('\r')) {
        let regions = h.highlight_line(line, ss).unwrap();
        let (colors, words): (Vec<_>, Vec<_>) =
            regions.into_iter().map(|(a, b)| (a.foreground, b)).unzip();
        for i in 0..words.len() {
            let r = colors[i].r;
            let g = colors[i].g;
            let b = colors[i].b;
            let word = words[i];
            let escaped = escape_latex_text(word.to_string());
            result.push(format!("\\textcolor[RGB]{{{r},{g},{b}}}{{{escaped}}}"));
        }
        result.push("\n".to_string());
    }
    result.push("\\end{Verbatim}\n".to_string());
    result.push(r"\end{tcolorbox}".to_string());

    let mut json = result
        .iter()
        .map(|s| Value::String(s.to_string()))
        .collect::<Vec<_>>();

    json.push(import!(r"\usepackage{fancyvrb}"));
    json.push(import!(r"\usepackage{tcolorbox}"));
    json.push(import!(VERBATIM_OVERRIDE_LATEX));
    serde_json::to_string(&json).unwrap()
}

fn escape_latex_text(text: String) -> String {
    let s = text
        .split('\\')
        .map(|t| t.replace('{', r"\{").replace('}', r"\}"))
        .collect::<Vec<String>>()
        .join(r"\textbackslash{}")
        .replace('#', r"\#")
        .replace('$', r"\$")
        .replace('%', r"\%")
        .replace('&', r"\&")
        .replace('_', r"\_")
        .replace('<', r"\textless{}")
        .replace('>', r"\textgreater{}")
        .replace('~', r"\textasciitilde{}")
        .replace('^', r"\textasciicircum{}");
    s
}

fn latex_inline(text: &str) -> String {
    let mut result = String::new();
    result.push('[');
    write!(result, r#"{{"name": "raw", "data": "\\verb|"}},"#,).unwrap();
    write!(
        result,
        "{},",
        serde_json::to_string(&text.replace('|', "\\|")).unwrap()
    )
    .unwrap();
    write!(result, r#"{{"name": "raw", "data": "|"}}"#,).unwrap();
    result.push(']');

    result
}

fn get_style_html(
    inline: &bool,
    font_size: u64,
    tab_size: u64,
    bg: &String,
    default_bg: Option<Color>,
) -> String {
    let padding = if *inline { "0.1" } else { "0.5" };
    let hex = if bg == "default" && default_bg.is_some() {
        let c = default_bg.unwrap();
        format!("{:02x}{:02x}{:02x}", c.r, c.g, c.b)
    } else {
        bg.clone()
    };

    let mut style = String::from("style=\"");
    write!(style, "box_sizing: border_box; ").unwrap();
    write!(style, "padding: {padding}rem; ").unwrap();
    write!(style, "tab-size: {tab_size}; ").unwrap();
    write!(style, "font-size: {font_size}px; ").unwrap();
    write!(style, "background-color: #{hex}; ").unwrap();

    style.push('\"');
    style
}

fn get_highlighted_html(
    code: &str,
    theme: &Theme,
    syntax: &SyntaxReference,
    ss: &SyntaxSet,
) -> (String, Option<Color>) {
    let mut h = HighlightLines::new(syntax, theme);
    let incl_bg = IncludeBackground::No;
    let mut html: Vec<String> = vec![];

    // avoiding lines() here because we want to include the final newline
    for line in code.split('\n').map(|s| s.trim_end_matches('\r')) {
        let regions = h.highlight_line(line, ss).unwrap();
        html.push(styled_line_to_highlighted_html(&regions[..], incl_bg).unwrap())
    }
    (html.join("<br>"), theme.settings.background)
}
