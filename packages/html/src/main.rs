use std::{
    env,
    fmt::Write,
    io::{self, Read},
};

use serde_json::{from_str, json, Value};

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

    let Some(action) = args.get(0) else {
        eprintln!("No action was provided.");
        return;
    };

    match action.as_str() {
        "manifest" => print!("{}", &manifest()),
        "transform" => {
            let from = args.get(1).unwrap();
            let format = args.get(2).unwrap();

            if "html" != format {
                eprintln!("Output format not supported");
                return;
            }

            print!("{}", transform(from));
        }
        other => eprintln!("Invalid action '{other}'"),
    }
}

fn transform(from: &str) -> String {
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        from_str(&buffer).unwrap()
    };

    match from {
        "__bold" => transform_tag(input, "strong"),
        "__italic" => transform_tag(input, "em"),
        "__superscript" => transform_tag(input, "sup"),
        "__subscript" => transform_tag(input, "sub"),
        "__underlined" => transform_tag(input, "u"),
        "__strikethrough" => transform_tag(input, "del"),
        "__paragraph" => transform_tag(input, "p"),
        "__document" => transform_document(input),
        "__text" => escape_text(input),
        "__heading" => transform_heading(input),
        "__error" => transform_error(input),
        _ => panic!("element not supported"),
    }
}

fn transform_document(doc: Value) -> String {
    let mut result = String::new();
    result.push('[');

    write!(result, "{},", raw!("<html><head><title>Document</title>")).unwrap();

    write!(result, "{},", raw!("<style>")).unwrap();
    write!(result, "{},", raw!(include_str!("templates/html.css"))).unwrap();
    write!(result, "{},", raw!("</style></head><body>")).unwrap();

    if let Value::Array(children) = &doc["children"] {
        for child in children {
            result.push_str(&serde_json::to_string(child).unwrap());
            result.push(',');
        }
    }

    write!(result, "{}", raw!("</body></html>")).unwrap();
    result.push(']');

    result
}

fn transform_heading(heading: Value) -> String {
    let mut result = String::new();
    result.push('[');

    let Value::String(s) = &heading["arguments"]["level"] else {
        panic!();
    };
    let level = s.parse::<u8>().unwrap().clamp(1, 6);

    write!(result, "{},", raw!(format!("<h{level}>"))).unwrap();

    if let Value::Array(children) = &heading["children"] {
        for child in children {
            result.push_str(&serde_json::to_string(child).unwrap());
            result.push(',');
        }
    }

    write!(result, "{}", raw!(format!("</h{level}>"))).unwrap();
    result.push(']');

    result
}

fn transform_error(error: Value) -> String {
    let mut result = String::new();
    result.push('[');

    let Value::String(source) = &error["arguments"]["source"] else {
        panic!();
    };
    let Value::String(input) = &error["arguments"]["input"] else {
        panic!();
    };
    let Value::String(err) = &error["data"] else {
        panic!();
    };

    // TODO: Maybe make these errors look better. Be careful though, see notes in API, don't use
    //   calls to other modules that may fail. I have taken care to not use __text but rather just
    //   entered the text myself, because if I used __text and that failed, it would lead to
    //   infinite recursion, which is bad
    write!(result, "{},", raw!(r#"<span style="display: inline-block; background:#ffebeb; padding: 0.5rem; color: black; border-radius: 0.3rem; box-shadow: 0 0 2px #0000003b;">"#)).unwrap();

    let data = escape(format!("Error originating from {source}: {err} on input {input}").as_str());
    write!(result, "{},", raw!(data)).unwrap();

    write!(result, "{}", raw!("</span>")).unwrap();

    result.push(']');

    result
}

fn escape_text(module: Value) -> String {
    if let Value::String(s) = &module["data"] {
        let s = escape(s);
        format!("[{}]", raw!(s).to_string())
    } else {
        panic!("Malformed text module");
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn transform_tag(node: Value, html_tag: &str) -> String {
    let mut result = String::new();
    result.push('[');

    write!(result, "{},", raw!(format!("<{html_tag}>"))).unwrap();

    if let Value::Array(children) = &node["children"] {
        for child in children {
            result.push_str(&serde_json::to_string(child).unwrap());
            result.push(',');
        }
    }

    write!(result, "{}", raw!(format!("</{html_tag}>"))).unwrap();
    result.push(']');

    result
}

fn manifest() -> String {
    serde_json::to_string(&json!(
        {
            "version": "0.1",
            "name": "html",
            "description": "This packages provides HTML support for the basic Modmark features.",
            "transforms": [
                {
                    "from": "__bold",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__italic",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__superscript",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__subscript",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__strikethrough",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__underlined",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__document",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__text",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__paragraph",
                    "to": ["html"],
                    "arguments": [],
                },
                {
                    "from": "__error",
                    "to": ["html"],
                    "arguments": [
                    {
                        "name":"source",
                        "description":"Source for the error",
                        "default":"<unknown>"
                    },
                    {
                        "name":"target",
                        "description":"Target for the error",
                        "default":"<unknown>"
                    },
                    {
                        "name":"input",
                        "description":"Input for the error",
                        "default":"<unknown>"
                    },
                ],
                },
                {
                  "from": "__heading",
                    "to": ["html"],
                    "arguments": [
                        {
                            "name": "level",
                            "description": "The level of the heading",
                            "default": "1"
                        }
                    ],
                },

            ]
        }
    ))
    .unwrap()
}
