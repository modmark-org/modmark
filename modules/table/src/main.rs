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
        "name": "Standard table module",
        "version": "0.1",
        "description": "This package supports [table] modules",
        "transforms": [
            {
                "from": "table",
                "to": ["html"],
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
    match to.as_str() {
        "html" => {
            let input: Value = {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).unwrap();
                serde_json::from_str(&buffer).unwrap()
            };

            let delimiter = input["arguments"]
                .get("col_delimiter")
                .map(|val| serde_json::to_string(val).unwrap())
                .unwrap_or_else(|| "|".to_string());

            //FIXME read delimiter from args
            let rows: Vec<Vec<&str>> = input["data"]
                .as_str()
                .unwrap()
                .lines()
                .map(|row| row.split(&delimiter).collect())
                .collect();

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
        other => {
            eprintln!("Cannot convert table to {other}");
            return;
        }
    }
}