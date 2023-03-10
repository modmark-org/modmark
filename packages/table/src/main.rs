use std::convert::TryInto;
use std::env;
use std::io::{self, Read};

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

// Entry point
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
        json!(
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
        )
    );
}

fn transform(from: &str, to: &str) {
    match from {
        "table" => transform_table(to),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

fn transform_table(to: &str) {
    // We make sure to exit early if invalid format, not to do unnecessary calculations
    if to != "latex" && to != "html" {
        eprintln!("Unsupported format {to}, only HTML and LaTeX are supported!");
        return;
    }

    // We read stdin
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };

    // Try to parse it as a table. If none, parse_table made sure to log the errors and we exit
    // without returning valid JSON
    let Some(table) = parse_table(&input) else {
        return;
    };

    // If table was valid, execute! (also, we know that we have nothing else than latex/html so
    // anything else is unreachable)
    match to {
        "html" => println!("{}", table.to_html()),
        "latex" => println!("{}", table.to_latex()),
        _ => unreachable!(),
    }
}

// The alignment for all columns in the table. The alignment can be set to either the same value for
// all columns (entering "right" makes it be right-adjusted in all columns), represented by
// Alignment::All(...), or it might be different for each column, represented by Alignment::Columns
#[derive(PartialEq, Eq, Debug)]
enum Alignment {
    All(ColumnAlignment),
    Columns(Vec<ColumnAlignment>),
}

impl Alignment {
    // This gets the LaTeX alignment string. If we have a left-aligned column followed by an
    // right-aligned column, and we use borders, we get |l|r|. Width is used for when we have
    // Alignment::All
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

    // Gets the alignment for one specific column. May panic if that column doesn't exist
    fn for_column(&self, idx: usize) -> &ColumnAlignment {
        match self {
            Alignment::All(c) => c,
            Alignment::Columns(c) => &c[idx],
        }
    }

    // Gets the width of this Alignment configuration, if it has a fixed one
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
    // This goes from the argument (like "left" or "llcc") to the Alignment, if possible,
    // otherwise errors unit error
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
                .map(Columns),
        }
    }
}

// Defines the alignment for one specific column
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

// The different border options that exist. All these options implies "outer" borders but the None
// option, so horizontal contains vertical outer borders
#[derive(PartialEq, Eq, Debug)]
enum Borders {
    All,
    Horizontal,
    Vertical,
    Outer,
    None,
}

// Argument to value
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

// The struct holding a table. Since the text contained within holds pointers to stdin, we need
// lifetimes to avoid copying
#[derive(Debug)]
#[allow(dead_code)]
struct Table<'a> {
    width: usize,
    height: usize,
    content: Vec<Vec<&'a str>>,
    alignment: Alignment,
    borders: Borders,
    header: bool,
}

impl Table<'_> {
    // Turns this table to LaTeX and gets a JSON value (containing mostly raw stuff) to return
    fn to_latex(&self) -> Value {
        let mut vec: Vec<Value> = vec![];

        let col_key = if self.width == 0 {
            // If we have an empty table, we need some alignment still since otherwise
            // we get an latex error
            "|l|".to_string()
        } else if self.borders == Borders::All || self.borders == Borders::Vertical {
            self.alignment.latex_str(self.width, true)
        } else if self.borders == Borders::None {
            self.alignment.latex_str(self.width, false)
        } else {
            format!("|{}|", self.alignment.latex_str(self.width, false))
        };

        vec.push(raw!("\\begin{center}\n"));
        vec.push(raw!(format!("\\begin{{tabular}} {{ {} }}\n", col_key)));

        // Only "None" borders should not have top row
        if self.borders != Borders::None {
            vec.push(raw!("\\hline\n"));
        }

        // Loop though all rows
        for (idx, row) in self.content.iter().enumerate() {
            // Collect all inline_content values, if heading add bold tags **
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

            // For each cell in the row, push it and add & between, and \\\n to the end
            for (idx, val) in values.into_iter().enumerate() {
                if idx != 0 {
                    vec.push(raw!(" & "));
                }
                vec.push(val);
            }
            vec.push(raw!(" \\\\\n"));

            // If we should have a border in-between all rows, add it
            if self.borders == Borders::All || self.borders == Borders::Horizontal {
                vec.push(raw!("\\hline\n"));
            }
        }

        // Both horizontal and all already added this line, so it only needs to be
        // added on outer and vertical
        if self.borders == Borders::Outer || self.borders == Borders::Vertical {
            vec.push(raw!("\\hline\n"))
        }
        vec.push(raw!("\\end{tabular}\n"));
        vec.push(raw!(r"\end{center}"));
        json!(vec)
    }

    // Turns this table to HTML and gets a JSON value (containing mostly raw stuff) to return
    fn to_html(&self) -> Value {
        let mut vec: Vec<Value> = vec![];
        // Push opening tag, border style on table if outer borders
        if self.borders == Borders::None {
            vec.push(raw!("<table>"));
        } else {
            vec.push(raw!(
                r#"<table style="border: 1px solid black; border-collapse: collapse;">"#
            ));
        }

        // Here is the style for all th/td elements
        let inside_border_style = if self.borders == Borders::All {
            " border: 1px solid black; border-collapse: collapse;"
        } else if self.borders == Borders::Vertical {
            " border-left: 1px solid black; border-right: 1px solid black; border-collapse: collapse;"
        } else if self.borders == Borders::Horizontal {
            " border-top: 1px solid black; border-bottom: 1px solid black; border-collapse: collapse;"
        } else {
            ""
        };

        // Loop though each row
        for (idx, row) in self.content.iter().enumerate() {
            vec.push(raw!("<tr>"));

            // If it is the header, use th, else use td
            if idx == 0 && self.header {
                for (idx, elem) in row.iter().enumerate() {
                    let alignment = self.alignment.for_column(idx).html_style();
                    vec.push(raw!(format!(
                        r#"<th style="{alignment}{inside_border_style}">"#
                    )));
                    vec.push(inline_content!(elem));
                    vec.push(raw!("</th>"));
                }
            } else {
                for (idx, elem) in row.iter().enumerate() {
                    let alignment = self.alignment.for_column(idx).html_style();
                    vec.push(raw!(format!(
                        r#"<td style="{alignment}{inside_border_style}">"#
                    )));
                    vec.push(inline_content!(elem));
                    vec.push(raw!("</td>"));
                }
            }

            vec.push(raw!("</tr>"));
        }

        vec.push(raw!("</table>"));
        json!(vec)
    }
}

// Parses the JSON input to a table, if possible. Warnings/errors are printed out when running this.
// Many of the arguments uses try_into() to optionally get a Border, Alignment etc
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

    if height == 0 {
        eprintln!("Empty table");
        return Some(Table {
            width: 0,
            height,
            content,
            alignment,
            borders,
            header,
        });
    }

    // We get the max width
    let width = content.iter().map(|r| r.len()).max().unwrap();
    // If any row differ from this, it is jagged and we make sure to resize the arrays
    if content.iter().any(|r| r.len() != width) {
        eprintln!("The table is jagged; some rows are wider than others.");
        content.iter_mut().for_each(|r| r.resize(width, ""))
    }

    // This is an fatal error since we don't want users that have specified column-by-column to get
    // completely overwritten, so we fail here
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

// This function splits a text by a given delimiter, taking into account backslash escaping and so
// on. It may also trim the cells if the trim argument is true.
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
                input[start_idx..idx].trim()
            } else {
                &input[start_idx..idx]
            };

            res.push(str);
            start_idx = idx + delimiter.len();
        }
    }

    if start_idx != input.as_bytes().len() {
        let str = if trim {
            input[start_idx..].trim()
        } else {
            &input[start_idx..]
        };

        res.push(str)
    }
    res
}
