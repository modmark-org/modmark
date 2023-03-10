use std::convert::TryInto;
use std::env;
use std::fmt::Write;
use std::io::{self, Read};
use std::iter::repeat;
use std::process::exit;

use serde_json::{json, Value};

macro_rules! raw {
    ($expr:expr) => {
        json!({
            "name": "raw",
            "data": $expr
        })
    }
}

macro_rules! inline_content {
    ($expr:expr) => {
        json!({
            "name": "inline_content",
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
    print!("{}", serde_json::to_string(&json!(
        {
        "name": "table",
        "version": "0.1",
        "description": "This package supports [table] modules",
        "transforms": [
            {
                "from": "table",
                "to": ["html", "latex"],
                "arguments": [
                    {"name": "header", "default": "none", "description": "Style to apply to heading, none/bold"},
                    {"name": "alignment", "default": "left", "description": "Horizontal alignment in cells, left/center/right or l/c/r for each column"},
                    {"name": "borders", "default": "all", "description": "Which borders to draw, all/horizontal/vertical/outer/none"},
                    {"name": "delimiter", "default": "|", "description": "The delimiter between cells"},
                    {"name": "strip_whitespace", "default": "true", "description": "true/false to strip/don't strip whitespace in cells"}
                ],
            }
        ]
        }
    ))
    .unwrap());
}

fn transform(from: &String, to: &String) {
    match from.as_str() {
        "table" => transform_table(to),
        other => {
            eprintln!("Package does not support {other}");
            return;
        }
    }
}

#[derive(PartialEq, Debug)]
enum Target {
    HTML,
    LaTeX,
}

impl TryFrom<&str> for Target {
    type Error = ();
    fn try_from(str: &str) -> Result<Self, Self::Error> {
        let str = str.to_ascii_lowercase();
        if &str == "html" {
            Ok(Target::HTML)
        } else if &str == "latex" {
            Ok(Target::LaTeX)
        } else {
            Err(())
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
enum Alignment {
    All(ColumnAlignment),
    Columns(Vec<ColumnAlignment>),
}

impl Alignment {
    fn latex_str(&self, width: usize, border: bool) -> String {
        match self {
            Alignment::All(alignment) => {
                if border {
                    "|".to_string()
                        + (alignment.latex_char().to_string() + "|")
                            .repeat(width)
                            .as_str()
                } else {
                    alignment.latex_char().to_string().repeat(width)
                }
            }
            Alignment::Columns(vec) => {
                if border {
                    format!(
                        "|{}",
                        vec.iter()
                            .map(|a| format!("{}|", a.latex_char()))
                            .collect::<String>()
                    )
                } else {
                    vec.iter().map(|a| a.latex_char()).collect()
                }
            }
        }
    }

    fn for_column(&self, idx: usize) -> &ColumnAlignment {
        match self {
            Alignment::All(c) => c,
            Alignment::Columns(c) => &c[idx],
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
enum ColumnAlignment {
    Left,
    Center,
    Right,
}

impl ColumnAlignment {
    fn latex_char(&self) -> char {
        match self {
            ColumnAlignment::Left => 'l',
            ColumnAlignment::Center => 'c',
            ColumnAlignment::Right => 'r',
        }
    }

    fn html_style(&self) -> &str {
        match self {
            ColumnAlignment::Left => "text-align: left;",
            ColumnAlignment::Center => "text-align: center;",
            ColumnAlignment::Right => "text-align: right;",
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
enum Borders {
    All,
    Horizontal,
    Vertical,
    Outer,
    None,
}

impl TryFrom<&str> for Borders {
    type Error = ();
    fn try_from(str: &str) -> Result<Self, Self::Error> {
        use Borders::*;

        match str.to_ascii_lowercase().as_str() {
            "all" => Ok(All),
            "horizontal" => Ok(Horizontal),
            "vertical" => Ok(Vertical),
            "outer" => Ok(Outer),
            "none" => Ok(None),
            _ => Err(()),
        }
    }
}

impl Alignment {
    fn width(&self) -> Option<usize> {
        if let Alignment::Columns(vec) = self {
            Some(vec.len())
        } else {
            None
        }
    }
}

impl TryFrom<&str> for Alignment {
    type Error = ();
    fn try_from(str: &str) -> Result<Self, Self::Error> {
        use Alignment::*;
        use ColumnAlignment::*;

        match str.to_ascii_lowercase().as_str() {
            "left" => Ok(All(Left)),
            "center" => Ok(All(Center)),
            "right" => Ok(All(Right)),
            s => s
                .chars()
                .map(|c| match c.to_ascii_lowercase() {
                    'l' => Ok(Left),
                    'c' => Ok(Center),
                    'r' => Ok(Right),
                    _ => Err(()),
                })
                .collect::<Result<Vec<ColumnAlignment>, ()>>()
                .map(|v| Columns(v)),
        }
    }
}

#[derive(Debug)]
struct Table<'a> {
    width: usize,
    height: usize,
    content: Vec<Vec<&'a str>>,
    alignment: Alignment,
    borders: Borders,
    header: bool,
}

impl Table<'_> {
    fn to_latex(&self) -> Vec<Value> {
        let mut vec: Vec<Value> = vec![];

        let col_key = if self.borders == Borders::All || self.borders == Borders::Vertical {
            self.alignment.latex_str(self.width, true)
        } else if self.borders == Borders::None {
            self.alignment.latex_str(self.width, false)
        } else {
            format!("|{}|", self.alignment.latex_str(self.width, false))
        };

        vec.push(raw!("\\begin{center}\n"));
        vec.push(raw!(format!("\\begin{{tabular}} {{ {} }}\n", col_key)));

        if self.borders != Borders::None {
            vec.push(raw!("\\hline\n"));
        }

        for (idx, row) in self.content.iter().enumerate() {
            let values = if idx == 0 && self.header {
                row.iter()
                    .map(|c| format!("**{c}**"))
                    .map(|c| inline_content!(c))
                    .collect::<Vec<Value>>()
            } else {
                row.iter()
                    .map(|c| inline_content!(c))
                    .collect::<Vec<Value>>()
            };

            for (idx, val) in values.into_iter().enumerate() {
                if idx != 0 {
                    vec.push(raw!(" & "));
                }
                vec.push(val);
            }
            vec.push(raw!(" \\\\\n"));

            if self.borders == Borders::All || self.borders == Borders::Horizontal {
                vec.push(raw!("\\hline\n"));
            }
        }

        if self.borders == Borders::Outer {
            vec.push(raw!(r"\hline"))
        }
        vec.push(raw!("\\end{tabular}\n"));
        vec.push(raw!(r"\end{center}"));
        vec
    }

    fn to_html(&self) -> Vec<Value> {
        let mut vec: Vec<Value> = vec![];
        if self.borders == Borders::None {
            vec.push(raw!("<table>"));
        } else {
            vec.push(raw!(
                r#"<table style="border: 1px solid black; border-collapse: collapse;""#
            ));
        }

        let inside_border_style = if self.borders == Borders::All {
            "border: 1px solid black; border-collapse: collapse;"
        } else if self.borders == Borders::Vertical {
            "border-left: 1px solid black; border-right: 1px solid black; border-collapse: collapse;"
        } else if self.borders == Borders::Horizontal {
            "border-top: 1px solid black; border-bottom: 1px solid black; border-collapse: collapse;"
        } else {
            ""
        };

        let ths = format!("<th{inside_border_style}>");
        let tds = format!("<td{inside_border_style}>");

        for (idx, row) in self.content.iter().enumerate() {
            vec.push(raw!("<tr>"));

            if idx == 0 && self.header {
                for (idx, elem) in row.iter().enumerate() {
                    let alignment = self.alignment.for_column(idx).html_style();
                    vec.push(raw!(format!(r#"<th style="{alignment}{inside_border_style}">"#)));
                    vec.push(inline_content!(elem));
                    vec.push(raw!("</th>"));
                }
            } else {
                for (idx, elem) in row.iter().enumerate() {
                    let alignment = self.alignment.for_column(idx).html_style();
                    vec.push(raw!(format!(r#"<td style="{alignment}{inside_border_style}">"#)));
                    vec.push(inline_content!(elem));
                    vec.push(raw!("</td>"));
                }
            }

            vec.push(raw!("</tr>"));
        }

        vec.push(raw!("</table>"));
        vec
    }
}

fn parse_table(input: &Value) -> Option<Table> {
    let delimiter = input["arguments"]["delimiter"].as_str().unwrap();
    if delimiter.contains('\\') {
        eprintln!("The delimiter may not contain backslashes");
        return None;
    }

    let borders = match input["arguments"]["borders"].as_str().unwrap().try_into() {
        Ok(border) => border,
        Err(()) => {
            eprintln!("Invalid 'borders' arg, choose one of all/horizontal/vertical/outer/none");
            Borders::All
        }
    };

    let alignment = match input["arguments"]["alignment"].as_str().unwrap().try_into() {
        Ok(alignment) => alignment,
        Err(()) => {
            eprintln!(
                "Invalid 'alignment' arg, choose one of left/center/right or l/c/r for each column"
            );
            Alignment::All(ColumnAlignment::Left)
        }
    };

    let strip_whitespace = match input["arguments"]["strip_whitespace"]
        .as_str()
        .unwrap()
        .to_ascii_lowercase()
        .as_str()
    {
        "false" => false,
        s => {
            if s != "true" {
                eprintln!("Invalid 'strip_whitespace' arg, choose one of true/false");
            }
            true
        }
    };

    let header = match input["arguments"]["header"]
        .as_str()
        .unwrap()
        .to_ascii_lowercase()
        .as_str()
    {
        "none" => false,
        s => {
            if s != "bold" {
                eprintln!("Invalid 'header' arg, choose one of none/bold");
            }
            true
        }
    };

    let body = input["data"].as_str().unwrap();
    let mut content = parse_content(body, delimiter, strip_whitespace);
    let height = content.len();
    //TODO: Fix special case where height = 0
    let width = content.iter().map(|r| r.len()).max().unwrap();
    if content.iter().any(|r| r.len() != width) {
        eprintln!("The table is jagged; some rows are wider than others.");
        content.iter_mut().for_each(|r| r.resize(width, ""))
    }

    if let Some(w) = alignment.width() {
        if w != width {
            eprintln!("Alignment given for {} columns but {} exist", w, width);
            return None;
        }
    }

    Some(Table {
        width,
        height,
        content,
        alignment,
        borders,
        header,
    })
}

fn parse_content<'a>(input: &'a str, delimiter: &'a str, trim: bool) -> Vec<Vec<&'a str>> {
    input
        .lines()
        .map(|row| split_by_delimiter(row, delimiter, trim))
        .collect()
}

fn split_by_delimiter<'a>(input: &'a str, delimiter: &'a str, trim: bool) -> Vec<&'a str> {
    let mut res = vec![];
    let mut escaped = false;
    let mut start_idx = 0;

    for (idx, c) in input.char_indices() {
        if escaped || idx < start_idx {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if input[idx..].starts_with(delimiter) {
            let str = if trim {
                &input[start_idx..idx].trim()
            } else {
                &input[start_idx..idx]
            };

            res.push(str);
            start_idx = idx + delimiter.len();
        }
    }

    if start_idx != input.as_bytes().len() {
        let str = if trim {
            &input[start_idx..].trim()
        } else {
            &input[start_idx..]
        };

        res.push(str)
    }
    res
}

fn transform_table(to: &str) {
    let Ok(format): Result<Target, _> = to.try_into() else {
        eprintln!("Unsupported format {to}, only HTML and LaTeX are supported!");
        return;
    };

    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };

    let Some(table) = parse_table(&input) else {
        return;
    };

    if format == Target::LaTeX {
        println!("{}", json! {table.to_latex()});
    } else if format == Target::HTML {
        println!("{}", json! {table.to_html()});
    }
}
