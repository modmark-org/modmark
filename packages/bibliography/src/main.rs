use std::{env, fs};
use std::borrow::Cow;
use std::io::{self, Read};

use hayagriva::Entry;
use hayagriva::style::{
    Apa, BibliographyStyle, ChicagoAuthorDate, Citation, CitationStyle, Database, DisplayString,
    Formatting, Ieee, Mla, Numerical,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

macro_rules! module {
    ($name:expr, $data:expr $(,$($args:tt)*)?) => {json!({"name": $name $(,"arguments":$($args)*)*, "data": $data})}
}

macro_rules! push_citation {
    ($e:expr) => {module!("list-push", $e, {"name": "inline_citations"})}
}

macro_rules! add_citation_label {
    ($a:expr, $b:expr) => {module!("set-add", format!("{}{}", $a, $b), {"name": "inline_citation_labels"})}
}

macro_rules! cite_label {
    ($e:expr) => {
        module!("cite-internal-do-not-use", $e)
    };
}

macro_rules! text {
    ($e:expr) => {
        module!("__text", $e)
    };
}

fn manifest() {
    print!(
        "{}",
        json!(
            {
            "name": "bibliography",
            "version": "0.1",
            "description": "This package supports bibliographies and in-text citations.",
            "transforms": [
                {
                    "from": "cite",
                    "to": ["any"],
                    "arguments": [
                        {"name": "note", "default": "", "description": "Note to show after citation, such as p. 3"}
                    ],
                    "description": "Add an inline citation to one of the sources defined in the bibliography. Example: [cite] modmark",
                    // "variables": {
                        // We really want this, but we can't since it would give cyclic dependencies
                        // together with unknown-content. So, we disable unknown-content
                        // "inline_citations": {"type": "list", "access": "push"}
                    // },
                    "unknown-content": true // This gives cyclic dependencies if enabled together
                    // with inline_citations/list-push, see below
                },
                {
                    "from": "cite-internal-do-not-use",
                    "to": ["any"],
                    "arguments": [],
                    "description": "Do not use this module",
                    "variables": {
                        "inline_citation_labels": {"type": "set", "access": "read"}
                    }
                },
                {
                    "from": "bibliography",
                    "to": ["any"],
                    "description": "Inserts a bibliography into the document. \
                    This module needs to exist for [cite]s to work properly. \
                    The bibliography may be read from the BibLaTeX or Hayagriva YAML format, \
                    from either the body of this module or a file passed in as the \"file\" argument.",
                    "arguments": [
                        {"name": "style", "default": "IEEE", "type": ["IEEE", "APA", "MLA", "CMoS"], "description": "The style to have the bibliography in"},
                        {"name": "file", "default": "", "description": "A file containing BibLaTeX or Hayagriva YAML with the bibliography"},
                        {"name": "visibility", "default": "shown", "type": ["shown", "hidden"], "description": "Whether the bibliography is 'shown' or 'hidden'. \
                        Note that a [bibliography] must exist for [cite]s to work, and if you don't want a bibliography in your document, you can set this argument to 'hidden'."}
                    ],
                    "variables": {
                        "inline_citations": {"type": "list", "access": "read"},
                        "inline_citation_labels": {"type": "set", "access": "add"},
                        "imports": {"type": "set", "access": "add"}
                    }
                }
            ]
            }
        )
    );
}

// Cyclic dependencies.
// Consider this document
// [cite] a
// [cite] b
// If cite is both unknown_content and push access to the list, it means a must happen before
// b (due to the order they appear in the document, list access is granular).
// a would expand to one [list-push] and something else, but then we have a [list-push] which is
// above b, which also requires list pushing to the same list. the [list-push] from a must thus
// occur before b, but also, b is unknown-content and so it must happen before a. Thus, we have
// a cyclic dependency.

fn transform(from: &str, to: &str) {
    let input: Value = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        serde_json::from_str(&buffer).unwrap()
    };

    match from {
        "cite" => transform_cite(to, &input),
        "cite-internal-do-not-use" => transform_cite_label(to, &input),
        "bibliography" => transform_bibliography(to, &input),
        other => {
            eprintln!("Package does not support {other}");
        }
    }
}

// The name Citation clashes with Hayagriva
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InlineCitation<'a> {
    key: &'a str,
    note: Option<&'a str>,
}

fn transform_cite(_to: &str, input: &Value) {
    let citation = {
        let key = input["data"].as_str().unwrap();
        let note = {
            let arg = input["arguments"]["note"].as_str().unwrap();
            (!arg.is_empty()).then_some(arg)
        };
        let citation = InlineCitation { key, note };
        serde_json::to_string(&citation).unwrap()
    };

    let output = json!([push_citation!(&citation), cite_label!(&citation)]);

    println!("{output}");
}

fn transform_cite_label(_to: &str, input: &Value) {
    let citation = input["data"].as_str().unwrap();

    let labels: Vec<String> = {
        let var = env::var("inline_citation_labels").unwrap_or("[]".to_string());
        serde_json::from_str(&var).unwrap()
    };

    if let Some(label) = labels.iter().find_map(|s| s.strip_prefix(citation)) {
        println!("{label}");
    } else {
        eprintln!("You must add a [bibliography] for [cite]s to work. If you don't want it visible, do [bibliography visibility=hidden]");
    }
}

fn transform_bibliography(_to: &str, input: &Value) {
    let Some(bibliography) = read_bibliography(input) else { return; };

    // This is the citations (i.e [cite]s) used in the text
    let citations: Vec<String> = {
        let var = env::var("inline_citations").unwrap_or("[]".to_string());
        serde_json::from_str(&var).unwrap()
    };

    // This is our database of all entries (like a db containing all biblatex-entries)
    let mut database = Database::from_entries(bibliography.iter());

    // What style to use (IEEE, APA etc)
    let style_arg = input["arguments"]["style"].as_str().unwrap();

    // Unwrap is safe since Core has checked enums already
    let (bibliography_style, mut citation_style) = get_styles(style_arg).unwrap();

    let mut output: Vec<Value> = vec![];

    for citation_str in &citations {
        // We parse the citation (which is a JSON obj)
        let citation: InlineCitation = serde_json::from_str(citation_str).unwrap();

        // We find the record by the key
        let entry = if let Some(record) = database.records.get(citation.key) {
            record.entry
        } else {
            eprintln!(
                "Missing citation {0}, mentioned in [cite] {0} but not defined in the bibliography",
                citation.key
            );
            output.push(add_citation_label!(
                citation_str,
                format!(
                    "[{}]",
                    text!(format!("[Missing citation '{}']", citation.key))
                )
            ));
            continue;
        };

        // We construct a new citation (representing [cite] in Hayagravia)
        let hayagriva_citation: Citation = Citation::new(entry, citation.note);

        // We cite that entry from our db
        let database_citation = database.citation(citation_style.as_mut(), &[hayagriva_citation]);

        // From this, we construct our citation string (what the [cite] should be turned into).
        // We pass this as JSON format to let Hayagravia apply formatting if needed (which it
        // doesn't, in this version at least). We encase it in [] for IEE and () for others
        let json_citation = if style_arg == "IEEE" {
            let mut vec = vec![text!("[")];
            vec.append(&mut display(&database_citation.display));
            vec.push(text!("]"));
            Value::Array(vec)
        } else {
            let mut vec = vec![text!("(")];
            vec.append(&mut display(&database_citation.display));
            vec.push(text!(")"));
            Value::Array(vec)
        };

        // Then, we are adding that citation as the label (that the [cite], now [cite-internal],
        // will turn into). Note, since we are doing string-matching later on (not value-based
        // matching), we want to make sure not to parse citation_str and re-stringify it since
        // object kv-pairs may change order. The string must match exactly to be picked up again
        output.push(add_citation_label!(
            citation_str,
            format!("{json_citation}")
        ));
    }

    let visibility = input["arguments"]["visibility"].as_str().unwrap();

    // If the bibliography should be shown, show it!
    if visibility == "shown" {
        for entry in database.bibliography(bibliography_style.as_ref(), None) {
            if let Some(prefix) = &entry.prefix {
                // If we have a prefix (which is the number in [] for IEEE), encase it in brackets
                output.push(text!("["));
                output.append(&mut display(prefix));
                output.push(text!("] "));
            }
            output.append(&mut display(&entry.display));
            output.push(module!("newline", ""));
        }
    }

    println!("{}", Value::Array(output));
}

/// Gets the styles for the given key, as a pair of the bibliography style and citation style. If
/// the key has no style mappings, None is returned. For the keys IEEE, APA, MLA and CMoS, Some is
/// always returned, and for any other key, None is returned.
fn get_styles(key: &str) -> Option<(Box<dyn BibliographyStyle>, Box<dyn CitationStyle>)> {
    let bibliography_style: Box<dyn BibliographyStyle> = match key {
        "IEEE" => Box::new(Ieee::new()),
        "APA" => Box::new(Apa::new()),
        "MLA" => Box::new(Mla::new()),
        "CMoS" => Box::new(ChicagoAuthorDate::new()),
        _ => return None,
    };

    let citation_style: Box<dyn CitationStyle> = match key {
        "IEEE" => Box::new(Numerical::new()),
        "APA" => Box::new(ChicagoAuthorDate::new()),
        "MLA" => Box::new(ChicagoAuthorDate::new()),
        "CMoS" => Box::new(ChicagoAuthorDate::new()),
        _ => return None,
    };

    Some((bibliography_style, citation_style))
}

/// This function reads the bibliography by first:
/// * Checking the 'file' argument, and if it is non-empty, reads that file
/// * Otherwise, checking the body of the module
///
/// Then, it parses the bibliography by first:
/// * Trying to parse it as a BibLaTeX file
/// * If that doesn't work, try to parse it as a Hayagriva Yaml file
///
/// If everything succeeds, the list of entries is returned, otherwise errors are
/// printed to stderr and None is returned. Parsing errors will print both BibLaTeX
/// and Yaml errors.
fn read_bibliography(input: &Value) -> Option<Vec<Entry>> {
    let filename = input["arguments"]["file"].as_str().unwrap();
    let input_string: Cow<'_, str> = if filename.is_empty() {
        Cow::Borrowed(input["data"].as_str().unwrap())
    } else if let Ok(content) = fs::read_to_string(filename) {
        Cow::Owned(content)
    } else {
        eprintln!("Could not read file {filename}");
        return None;
    };

    let bibliography = hayagriva::io::from_biblatex_str(&without_latex_comments(&input_string))
        .or_else(|e1| hayagriva::io::from_yaml_str(&input_string).map_err(|e2| (e1, e2)));

    match bibliography {
        Ok(b) => Some(b),
        Err((e1, e2)) => {
            eprintln!(
                "Could not parse bibliography as BibLaTeX: {}",
                e1.into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            eprintln!("Could not parse bibliography as Yaml: {e2}");
            None
        }
    }
}

/// This function displays a `DisplayString` by converting its formatting rules/spans
/// to JSON representations that can be picked up by `ModMark` again. It uses
/// __text for all text, wrapped in __bold and __italic for those rules, and uses
/// the `[link]` package for URLs
fn display(string: &DisplayString) -> Vec<Value> {
    let mut values = vec![];
    let mut last_unformatted = 0usize;
    for (range, rule) in &string.formatting {
        assert!(range.start >= last_unformatted);
        if range.start != last_unformatted {
            values.push(text!(
                string.value[last_unformatted..range.start].to_string()
            ));
        }
        last_unformatted = range.end;
        let substring = &string.value[range.clone()];
        let value = match rule {
            Formatting::Bold => {
                json!({"name": "__bold", "arguments": {}, "children": [text!(substring)]})
            }
            Formatting::Italic => {
                json!({"name": "__italic", "arguments": {}, "children": [text!(substring)]})
            }
            Formatting::Link(url) => module!("link", url, { "label": substring }),
        };
        values.push(value);
    }
    values.push(text!(&string.value[last_unformatted..]));
    values
}

fn without_latex_comments(string: &str) -> String {
    string.lines()
        .filter(|line| !line.trim_start().starts_with('%'))
        .collect::<Vec<_>>()
        .join("\n")
}
