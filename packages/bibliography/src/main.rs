use std::borrow::Cow;
use std::collections::HashSet;
use std::io::{self, Read};
use std::{env, fs};

use hayagriva::style::{
    Apa, BibliographyStyle, ChicagoAuthorDate, Citation, CitationStyle, Database, DisplayString,
    Formatting, Ieee, Mla, Numerical,
};
use hayagriva::Entry;
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

macro_rules! raw {
    ($content:expr) => {
        module!("raw", $content)
    };
}

macro_rules! import {
    ($package:expr) => {module!("set-add", $package, {"name": "imports"})}
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
                        // together with unknown-content. So, we disable this in favour of just using
                        // unknown-content
                        // "inline_citations": {"type": "list", "access": "push"}
                    // },
                    "unknown-content": true
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
                        {"name": "style", "default": "IEEE", "type": ["IEEE", "APA", "MLA", "Chicago"], "description": "The style to have the bibliography in"},
                        {"name": "file", "default": "", "description": "A file containing BibLaTeX or Hayagriva YAML with the bibliography"},
                        {"name": "visibility", "default": "visible", "type": ["visible", "hidden"], "description": "Whether the bibliography is 'visible' or 'hidden'. \
                        Note that a [bibliography] must exist for [cite]s to work, and if you don't want a bibliography in your document, you can set this argument to 'hidden'."},
                        {"name": "unused-entries", "default": "hidden", "type": ["visible", "hidden"], "description": "Whether unused entries in the database should be hidden or visible"}
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

// Why does cyclic dependencies occur?
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

fn transform_bibliography(to: &str, input: &Value) {
    let Some(bibliography) = read_bibliography(input) else { return; };

    // This is the citations (i.e [cite]s) used in the text
    let citations: Vec<String> = {
        let var = env::var("inline_citations").unwrap_or("[]".to_string());
        serde_json::from_str(&var).unwrap()
    };

    // This is our database of all entries
    let mut database = Database::from_entries(bibliography.iter());

    // What style to use (IEEE, APA etc)
    let style_arg = input["arguments"]["style"].as_str().unwrap();

    // Unwrap is safe since Core has checked enums already
    let (bibliography_style, mut citation_style) = get_styles(style_arg).unwrap();

    let mut output: Vec<Value> = vec![];
    let mut used_keys: HashSet<&str> = HashSet::new();

    if to == "latex" {
        output.push(import!(r"\usepackage[hidelinks]{hyperref}"));
    }

    let unused_keys_is_visible =
        input["arguments"]["unused-entries"].as_str().unwrap() == "visible";

    for citation_str in &citations {
        // We parse the citation (which is a JSON obj)
        let citation: InlineCitation = serde_json::from_str(citation_str).unwrap();

        // We find the record by the key
        let entry = if let Some(record) = database.records.get(citation.key) {
            if !unused_keys_is_visible {
                // If unused keys isn't visible, we need to keep track of the used keys
                // This if is used to not having to store all used keys if we aren't gonna use
                // them later
                used_keys.insert(citation.key);
            }
            record.entry
        } else {
            eprintln!(
                "Missing citation key '{0}', consider adding it to your bibliography",
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
        // doesn't, in this version at least). We encase it in [] for IEEE and () for others
        let mut json_citation_content = if style_arg == "IEEE" {
            let mut vec = vec![text!("[")];
            vec.append(&mut display(&database_citation.display));
            vec.push(text!("]"));
            vec
        } else {
            let mut vec = vec![text!("(")];
            vec.append(&mut display(&database_citation.display));
            vec.push(text!(")"));
            vec
        };

        // Apply nicer formatting for the note depending on the
        // output format
        let json_citation = match to {
            "latex" => {
                let mut vec = vec![raw!(format!(r"\hyperlink{{bibentry:{}}}{{", entry.key()))];
                vec.append(&mut json_citation_content);
                vec.push(raw!("}"));
                Value::Array(vec)
            }
            "html" => {
                let mut vec = vec![raw!(format!(r##"<a href="#bibentry:{}">"##, entry.key()))];
                vec.append(&mut json_citation_content);
                vec.push(raw!("</a>"));
                Value::Array(vec)
            }
            _ => Value::Array(json_citation_content),
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

    let is_visible = input["arguments"]["visibility"].as_str().unwrap() == "visible";

    // If the bibliography should be shown, show it!
    if is_visible && (unused_keys_is_visible || !used_keys.is_empty()) {
        output.append(&mut generate_bibliography(
            &database,
            bibliography_style.as_ref(),
            unused_keys_is_visible,
            &used_keys,
            to,
        ))
    }

    println!("{}", Value::Array(output));
}

/// Generate the bibliography that should be displayed
fn generate_bibliography<'a>(
    database: &Database<'a>,
    bibliography_style: &dyn BibliographyStyle<'a>,
    unused_keys_is_visible: bool,
    used_keys: &HashSet<&str>,
    to: &str,
) -> Vec<Value> {
    let entries = database
        .bibliography(bibliography_style, None)
        .into_iter()
        // Remove any unused keys if they shouldn't be visible
        .filter(|entry| unused_keys_is_visible || used_keys.contains(&entry.entry.key()))
        // If we have a prefix (which is the number in [] for IEEE), encase it in brackets
        .map(|entry| {
            let prefix = entry.prefix.as_ref().map(|p| {
                let mut vec = vec![text!("[")];
                vec.append(&mut display(p));
                vec.push(text!("]"));
                vec
            });
            (prefix, entry)
        });

    let mut output = vec![];

    match to {
        // We want to use a tabularx environment when outputing latex where the prefix
        // is in the first column and the rest in the second
        "latex" => {
            output.push(raw!("\n"));
            output.push(import!("\\usepackage{tabularx}"));

            output.push(raw!(
                r"\renewcommand{\arraystretch}{1.5}
\begin{tabularx}{\textwidth}{p{0.3cm} X}
"
            ));

            entries.for_each(|(prefix, entry)| {
                if let Some(mut prefix) = prefix {
                    output.append(&mut prefix);
                }

                output.push(raw!(format!(
                    r"& \leavevmode \hypertarget{{bibentry:{}}}{{",
                    entry.entry.key()
                )));
                output.append(&mut display(&entry.display));
                output.push(raw!("}".to_string()));
                output.push(module!("newline", ""));
                output.push(raw!("\n"));
            });

            output.push(raw!(
                r"\end{tabularx}
\renewcommand{\arraystretch}{1}
"
            ));
        }
        // For html output we use a simple css grid layout
        "html" => {
            output.push(raw!(
                r#"
<style>
    .modmark-bibliography {
        display: grid;
        grid-template-columns: auto 1fr;
        gap: 0.5rem;
    }
        
    .modmark-bibliography>.modmark-bibliography-prefix {
        grid-column: 1;
    }
        
    .modmark-bibliography>.modmark-bibliography-bibitem {
        grid-column: 2;
    }
</style>
<div class="modmark-bibliography">"#
            ));

            entries.for_each(|(prefix, entry)| {
                if let Some(mut prefix) = prefix {
                    output.push(raw!(r#"<span class="modmark-bibliography-prefix">"#));
                    output.append(&mut prefix);
                    output.push(raw!("</span>"));
                }

                output.push(raw!(format!(
                    r#" <span id="bibentry:{}">"#,
                    entry.entry.key()
                )));
                output.append(&mut display(&entry.display));
                output.push(raw!("</span>"));
                output.push(module!("newline", ""));
            });

            output.push(raw!("</div>"));
        }
        // if we don't have any special formatting, just fallback to some basic styling
        _ => {
            entries.for_each(|(prefix, entry)| {
                if let Some(mut prefix) = prefix {
                    output.append(&mut prefix);
                    output.push(text!(" "));
                }
                output.append(&mut display(&entry.display));
                output.push(module!("newline", ""));
            });
        }
    }

    output
}

/// Gets the styles for the given key, as a pair of the bibliography style and citation style. If
/// the key has no style mappings, None is returned. For the keys IEEE, APA, MLA and Chicago, Some is
/// always returned, and for any other key, None is returned.
fn get_styles(key: &str) -> Option<(Box<dyn BibliographyStyle>, Box<dyn CitationStyle>)> {
    let bibliography_style: Box<dyn BibliographyStyle> = match key {
        "IEEE" => Box::new(Ieee::new()),
        "APA" => Box::new(Apa::new()),
        "MLA" => Box::new(Mla::new()),
        "Chicago" => Box::new(ChicagoAuthorDate::new()),
        _ => return None,
    };

    let citation_style: Box<dyn CitationStyle> = match key {
        "IEEE" => Box::new(Numerical::new()),
        "APA" => Box::new(ChicagoAuthorDate::new()),
        "MLA" => Box::new(ChicagoAuthorDate::new()),
        "Chicago" => Box::new(ChicagoAuthorDate::new()),
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
        if !input["data"].as_str().unwrap().is_empty() {
            eprintln!("Bibliography contains body text but it is ignored since the 'file' argument is present");
        }
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
    string
        .lines()
        .filter(|line| !line.trim_start().starts_with('%'))
        .collect::<Vec<_>>()
        .join("\n")
}
