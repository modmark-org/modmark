use std::env;
use std::fmt::Write;
use std::io::{self, Read};

use serde_json::{json, Value};

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
        "name": "Standard table package",
        "version": "0.1",
        "description": "This package supports [table] modules",
        "transforms": [
            {
                "from": "table",
                "to": ["html", "latex"],
                "arguments": [
                    {"name": "col_delimiter", "default": "|", "description": "The string delimiter for columns"}
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

fn transform_table(to: &String) {
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };

    let Value::String(delimiter) = &input["arguments"]["col_delimiter"] else {
        panic!("No col_delimiter argument was provided");
    };

    let rows: Vec<Vec<&str>> = input["data"]
    .as_str()
    .unwrap()
    .lines()
    .map(|row| row.split(delimiter).collect())
    .collect();

    match to.as_str() {
        "html" => transform_html(rows),
        "latex" => transform_latex(rows),
        other => {
            eprintln!("Cannot convert table to {other}");
            return;
        }
    }
}

fn transform_latex(rows: Vec<Vec<&str>>) {
    let width = rows[0].len();

    let shape = "c".repeat(width);

    let mut output = String::new();
    output.push('[');
    write!(output, r#"{{"name": "raw", "data": "\\begin{{center}}\n\\begin{{tabular}}{{{shape}}}\n"}},"#).unwrap();
    
    

    for row in rows {
        let mut row = row.iter().peekable();
        while let Some(col) = row.next() {
            write!(output, r#"{{"name": "inline_content", "data": "{col}"}},"#).unwrap();
            if row.peek().is_some() {
                output.push_str(r#"{"name": "raw", "data": " & "},"#);
            }
        }
        output.push_str(r#"{"name": "raw", "data": "\\\\\n"},"#);
    }
    
    write!(output, r#"{{"name": "raw", "data": "\\end{{tabular}}\n\\end{{center}}\n"}}"#).unwrap();
    output.push(']');

    print!("{output}");
}

fn transform_html(rows: Vec<Vec<&str>>) {

    let mut output = String::new();
    output.push('[');
    output.push_str(r#"{"name": "raw", "data": "<table>"},"#);
    for row in rows {
        output.push_str(r#"{"name": "raw", "data": "<tr>"},"#);
        for col in row {
            output.push_str(r#"{"name": "raw", "data": "<td>"},"#);
            write!(output, r#"{{"name": "inline_content", "data": "{col}"}},"#).unwrap();
            output.push_str(r#"{"name": "raw", "data": "</td>"},"#);
        }
        output.push_str(r#"{"name": "raw", "data": "</tr>"},"#);
    }
    output.push_str(r#"{"name": "raw", "data": "</table>"}"#);
    output.push(']');

    print!("{output}");

}