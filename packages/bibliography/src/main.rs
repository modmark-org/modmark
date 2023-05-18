use std::borrow::Cow;
use std::collections::HashSet;
use std::io::{self, Read};
use std::{env, fs};

use hayagriva::style::{
    Apa, BibliographyStyle, ChicagoAuthorDate, Citation, CitationStyle, Database, DisplayReference,
    DisplayString, Formatting, Ieee, Mla, Numerical,
};
use hayagriva::types::EntryType::{Article, Web};
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
        json!($content)
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

macro_rules! text_or_reparse {
    ($e:expr, $reparse:expr) => {
        if $reparse {
            module!("inline_content", $e.replace("[Online]", r"\[Online]"))
        } else {
            module!("__text", $e)
        }
    };
}

fn manifest() {
    print!(
        "{}",
        json!(
            {
            "name": "bibliography",
            "version": "0.2",
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
                        {"name": "unused-entries", "default": "hidden", "type": ["visible", "hidden"], "description": "Whether unused entries in the database should be hidden or visible"},
                        {"name": "insertion-type", "default": "reparse", "type": ["reparse", "plain"], "description": "Whether to reparse the content in the bibliography using inline_content"},
                        {"name": "output", "default": "plain", "type": ["plain", "table"], "description": "Whether to output the result in plain text or in a [table]. Note that for a [table] to work, there must exist a [table] module with support for custom delimiters for the target language"},
                        {"name": "specialization", "default": "enable", "type": ["enable", "disable"], "description": "Enabling specialization will render the result more nicely in HTML and LaTeX"},
                        {"name": "styling", "default": "all", "description": "What styling options are available for the target language, as a comma-separated list, available: \
                        'bold', 'italic', 'url' (via the [link] module), 'target' (via [target]/[link]) to enable references within the document"}
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

macro_rules! clone_field {
    ($name:expr, $from:expr, $to:expr) => {
        if let Some(result) = $from.get($name) {
            $to.set($name, result.clone())
                .expect(concat!("Expected to be able to set the field ", $name));
        }
    };
}

macro_rules! clone_fields {
    ($from:expr => $to:expr,) => {};
    ($from:expr => $to:expr, $f1:expr, $($fs:expr,)*) => {{
        clone_field!($f1, $from, $to);
        clone_fields!($from => $to, $($fs,)*);
    }}
}

/// Configuration of how styling should be applied
struct StylingConfig<'a> {
    bib_style: Box<dyn BibliographyStyle<'a>>,
    cit_style: Box<dyn CitationStyle<'a>>,
    bold: bool,
    italic: bool,
    url: bool,
    target: bool,
    reparse: bool,
}

impl StylingConfig<'_> {
    /// This parses a `StylingConfig` from the `style`, `styling` and `reparse` arguments.
    fn from(style: &str, styling: &str, reparse: &str) -> Option<Self> {
        let (bs, cs) = get_styles(style)?;
        let reparse = &reparse.to_ascii_lowercase() == "reparse";

        let s = styling.to_ascii_lowercase();
        let res = if s.trim() == "all" {
            Self {
                bib_style: bs,
                cit_style: cs,
                bold: true,
                italic: true,
                url: true,
                target: true,
                reparse,
            }
        } else {
            Self {
                bib_style: bs,
                cit_style: cs,
                bold: s.contains("bold"),
                italic: s.contains("italic"),
                url: s.contains("url"),
                target: s.contains("target"),
                reparse,
            }
        };
        Some(res)
    }
}

fn transform_bibliography(to: &str, input: &Value) {
    // Read the bibliography, and if it fails, read_bibliography() has already printed an error
    let Some(mut bibliography) = read_bibliography(input) else { return; };

    // Replace 'web' with 'article'
    bibliography.iter_mut().for_each(|x| {
        if x.kind() == Web {
            let mut new = Entry::new(x.key(), Article);
            clone_fields!(x => new,
                "parent",
                "title",
                "location", "publisher" , "archive" , "archive-location",
                "author", "editor",
                "date",
                "affiliated",
                "organization" , "issn" , "isbn" , "doi" , "serial-number" , "note" ,
                "issue" , "edition",
                "volume" , "page-range",
                "volume-total" , "page-total" ,
                "time-range" ,
                "runtime" ,
                "url",
                "language",
            );
            *x = new;
        }
    });

    // This is the citations (i.e [cite]s) used in the text
    let citations: Vec<String> = {
        let var = env::var("inline_citations").unwrap_or("[]".to_string());
        serde_json::from_str(&var).unwrap()
    };

    // This is our database of all entries
    let mut database = Database::from_entries(bibliography.iter());

    // Check what styling options are available for us
    // Style has bib/cite style (IEEE, APA etc)
    // Styling has the features available, like bold, italics etc
    // OK to unwrap since MMCore checks enum
    let mut styling = StylingConfig::from(
        input["arguments"]["style"].as_str().unwrap(),
        input["arguments"]["styling"].as_str().unwrap(),
        input["arguments"]["insertion-type"].as_str().unwrap(),
    )
    .unwrap();

    // This is true if we fall back to table
    let fallback_table = input["arguments"]["output"].as_str().unwrap() == "table";
    // This is true if we allow specialization
    let specialization = input["arguments"]["specialization"].as_str().unwrap() == "enable";

    // This is true if we should show unused keys
    let unused_keys_is_visible =
        input["arguments"]["unused-entries"].as_str().unwrap() == "visible";

    let mut output: Vec<Value> = vec![];
    let mut used_keys: HashSet<&str> = HashSet::new();

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
        let database_citation =
            database.citation(styling.cit_style.as_mut(), &[hayagriva_citation]);

        // json_citation will be the thing that [cite-internal-do-not-use] will be turned into.
        // We either to it as a [link label=...] (if we support link/target) or
        // as just formatted text
        let json_citation = if styling.target {
            let citation_content =
                display_inline_content(&database_citation.display, &styling, true);
            vec![module!("link", format!("bibentry:{}", entry.key()), {
                "label": &citation_content
            })]
        } else {
            display_to_ast(&database_citation.display, &styling, true)
        };

        // Then, we are adding that citation as the label (that the [cite], now [cite-internal],
        // will turn into). Note, since we are doing string-matching later on (not value-based
        // matching), we want to make sure not to parse citation_str and re-stringify it since
        // object kv-pairs may change order. The string must match exactly to be picked up again
        output.push(add_citation_label!(
            citation_str,
            serde_json::to_string(&json_citation).unwrap()
        ));
    }

    let is_visible = input["arguments"]["visibility"].as_str().unwrap() == "visible";

    // If the bibliography should be shown, show it!
    if is_visible {
        // We filter out keys, if we should, and get the prefix and content
        let entries: Vec<(Option<DisplayString>, DisplayReference)> = database
            .bibliography(styling.bib_style.as_ref(), None)
            .into_iter()
            // Remove any unused keys if they shouldn't be visible
            .filter(|entry| unused_keys_is_visible || used_keys.contains(&entry.entry.key()))
            // Extract the prefix and content of the reference
            .map(|entry| (entry.prefix.clone(), entry))
            .collect();

        // Now to generating the actual bib. We check what type of bib we want to generate,
        // if we have specialization enabled and we target HTML or LaTeX, then generate
        // specialized, otherwise check fallback_table arg to see the fallback
        if !entries.is_empty() {
            let mut bib_out = if specialization && to == "html" {
                generate_specialized_html(&entries, &styling)
            } else if specialization && to == "latex" {
                generate_specialized_latex(&entries, &styling)
            } else if fallback_table {
                generate_table(&entries, &styling)
            } else {
                generate_plain(&entries, &styling)
            };
            output.append(&mut bib_out);
        }
    }

    println!("{}", Value::Array(output));
}

fn generate_plain(
    entries: &[(Option<DisplayString>, DisplayReference)],
    styling: &StylingConfig,
) -> Vec<Value> {
    let mut rows: Vec<Value> = vec![];
    for (prefix, entry) in entries {
        if let Some(x) = prefix {
            rows.append(&mut display_to_ast(x, styling, true));
            rows.push(text!(" "));
        }
        if styling.target {
            rows.push(module!(
                "target",
                display_inline_content(&entry.display, styling, false),
                { "name": format!("bibentry:{}", entry.entry.key()) }
            ));
        } else {
            rows.append(&mut display_to_ast(&entry.display, styling, false));
        }
        rows.push(module!("newline", ""));
    }
    rows
}

fn generate_table(
    entries: &[(Option<DisplayString>, DisplayReference)],
    styling: &StylingConfig,
) -> Vec<Value> {
    let has_prefix = entries.iter().any(|(x, _)| x.is_some());
    let delimiter = "!!!!TaBlE_dElImItEr!!!!";
    let mut rows: Vec<String> = vec![];
    if has_prefix {
        for (prefix, entry) in entries {
            let content = if styling.target {
                format!(
                    "[target name=\"bibentry:{}\"]!{}!",
                    entry.entry.key(),
                    display_inline_content(&entry.display, styling, false)
                )
            } else {
                display_inline_content(&entry.display, styling, false)
            };
            if let Some(prefix) = prefix.as_ref() {
                rows.push(format!(
                    r"{}{delimiter}{content}",
                    display_inline_content(prefix, styling, true)
                ));
            } else {
                rows.push(format!(r"{delimiter}{content}"));
            }
        }
    } else {
        for (_, entry) in entries {
            rows.push(display_inline_content(&entry.display, styling, false))
        }
    }

    vec![module!("table", rows.join("\n"), {
        "delimiter": delimiter,
        "borders": "none"
    })]
}

fn generate_specialized_html(
    entries: &[(Option<DisplayString>, DisplayReference)],
    styling: &StylingConfig,
) -> Vec<Value> {
    let mut using_prefix = false;
    let mut bibitems: Vec<Value> = entries
        .iter()
        .flat_map(|(prefix, entry)| {
            let mut item = Vec::new();
            if let Some(prefix) = prefix.as_ref() {
                using_prefix = true;
                item.push(raw!(r#"<span class="modmark-bibliography-prefix">"#));
                item.append(&mut display_to_ast(prefix, styling, true));
                item.push(raw!("</span>"));
            }

            item.push(raw!(r#"<span class="modmark-bibliography-bibitem">"#));
            item.push(module!(
                "target",
                display_inline_content(&entry.display, styling, false),
                { "name": format!("bibentry:{}", entry.entry.key()) }
            ));
            item.push(raw!("</span>"));
            item.push(module!("newline", ""));
            item
        })
        .collect();

    let mut output = Vec::new();

    // Use css grid to get a two column layout if we have a prefix (like [3]).
    output.push(raw!("<style>"));
    output.push(raw!(if using_prefix {
        include_str!("multicolumn.css")
    } else {
        include_str!("singlecolumn.css")
    }));
    output.push(raw!("</style>"));

    output.push(raw!(r#"<div class="modmark-bibliography">"#));
    output.append(&mut bibitems);
    output.push(raw!("</div>"));

    output
}

fn generate_specialized_latex(
    entries: &[(Option<DisplayString>, DisplayReference)],
    styling: &StylingConfig,
) -> Vec<Value> {
    let default_ds = DisplayString::default();

    let mut longest_prefix = 0;
    let mut bibitems: Vec<Value> = entries
        .iter()
        .flat_map(|(prefix, entry)| {
            longest_prefix = longest_prefix.max(prefix.as_ref().map_or(0, |x| x.value.len()));
            let mut item = vec![];

            item.push(raw!("\\item[{"));
            item.append(&mut display_to_ast(
                prefix.as_ref().unwrap_or(&default_ds),
                styling,
                true,
            ));
            item.push(raw!("}]\n"));

            if styling.target {
                item.push(import!("\\usepackage[hidelinks]{hyperref}"));
                item.push(raw!(format!(
                    "\\hypertarget{{bibentry:{}}}{{",
                    entry.entry.key()
                )));
            }
            item.append(&mut display_to_ast(&entry.display, styling, false));
            if styling.target {
                item.push(raw!("}\n\n"));
            }

            item
        })
        .collect();

    let mut output = vec![
        raw!("\n"),
        raw!("\\begin{thebibliography}{"),
        raw!("9".repeat(longest_prefix)),
        raw!("}\n"),
        raw!("\\phantomsection\\addcontentsline{toc}{chapter}{Bibliography}\n"),
        raw!("\\raggedright\n"),
    ];
    output.append(&mut bibitems);
    output.push(raw!("\n\\centering\n"));
    output.push(raw!("\\end{thebibliography}"));
    output
}

/// Gets the styles for the given key, as a pair of the bibliography style and citation style. If
/// the key has no style mappings, None is returned. For the keys IEEE, APA, MLA and Chicago, Some is
/// always returned, and for any other key, None is returned.
fn get_styles<'a>(
    key: &str,
) -> Option<(Box<dyn BibliographyStyle<'a>>, Box<dyn CitationStyle<'a>>)> {
    let bibliography_style: Box<dyn BibliographyStyle> = match key {
        "IEEE" => Box::new({
            let mut x = Ieee::new();
            x.abbreviate_journals = false;
            x
        }),
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
/// to `ModMark` source representations that can be picked up by `[inline_content]` again. It
/// returns the text, wrapped in ** and // for those rules, and uses `[link]` for URLs.
/// Note that the uses of these are conditional, based on the `StylingConfig` passed
/// If citation_brackets is set, and the string is non-empty, styling.cit_style.brackets() are
/// added (if they exist). Special care is taken to intersperse link labels with `\` so nothing is
/// re-parsed there.
fn display_inline_content(
    string: &DisplayString,
    styling: &StylingConfig,
    citation_brackets: bool,
) -> String {
    if string.is_empty() {
        return "".to_string();
    }

    // We know that this is going to be re-parsed. If we don't want it re-parsed, however, we
    // intersperse with \, since foo = \f\o\o in mdm
    let escape_if_needed = |s: &str| {
        // If we are to reparse the text, we don't really want to escape stuff, but we manually
        // escape [Online] since that is a common thing that we actually don't want reparsed
        if styling.reparse {
            s.replace("[Online]", r"\[Online]")
        } else {
            escape_by_interspersion(s)
        }
    };

    let mut accumulated = String::new();

    if citation_brackets {
        if let Some(lb) = styling.cit_style.brackets().left() {
            accumulated.push_str(&format!(r"\{lb}"));
        }
    }

    let mut last_unformatted = 0usize;
    for (range, rule) in &string.formatting {
        assert!(range.start >= last_unformatted);
        if range.start != last_unformatted {
            accumulated.push_str(&escape_if_needed(
                &string.value[last_unformatted..range.start],
            ));
        }
        last_unformatted = range.end;
        let substring = &string.value[range.clone()];
        let value = match rule {
            Formatting::Bold if styling.bold => format!("**{}**", escape_if_needed(substring)),
            Formatting::Italic if styling.italic => format!("//{}//", escape_if_needed(substring)),
            Formatting::Link(url) if styling.url => format!(
                "[link label=\"{}\"][{url}]",
                escape_by_interspersion(substring)
            ),
            _ => escape_if_needed(substring),
        };
        accumulated.push_str(value.as_str());
    }
    accumulated.push_str(&escape_if_needed(&string.value[last_unformatted..]));

    if citation_brackets {
        if let Some(rb) = styling.cit_style.brackets().right() {
            accumulated.push_str(&format!(r"\{rb}"));
        }
    }

    accumulated
}

/// This function displays a `DisplayString` by converting its formatting rules/spans
/// to JSON representations that can be picked up by `ModMark` again. It uses
/// __text for all text, wrapped in __bold and __italic for those rules, and uses
/// the `[link]` package for URLs
/// Note that the uses of these are conditional, based on the `StylingConfig` passed.
/// If citation_brackets is set, and the string is non-empty, styling.cit_style.brackets() are
/// added (if they exist). Special care is taken to intersperse link labels with `\` so nothing is
/// re-parsed there.
fn display_to_ast(
    string: &DisplayString,
    styling: &StylingConfig,
    citation_brackets: bool,
) -> Vec<Value> {
    if string.is_empty() {
        return vec![];
    }

    let mut values = vec![];

    if citation_brackets {
        if let Some(lb) = styling.cit_style.brackets().left() {
            values.push(text!(lb));
        }
    }

    let mut last_unformatted = 0usize;
    for (range, rule) in &string.formatting {
        assert!(range.start >= last_unformatted);
        if range.start != last_unformatted {
            values.push(text_or_reparse!(
                string.value[last_unformatted..range.start].to_string(),
                styling.reparse
            ));
        }
        last_unformatted = range.end;
        let substring = &string.value[range.clone()];
        let value = match rule {
            Formatting::Bold if styling.bold => {
                json!({"name": "__bold", "arguments": {}, "children": [text_or_reparse!(substring, styling.reparse)]})
            }

            Formatting::Italic if styling.italic => {
                json!({"name": "__italic", "arguments": {}, "children": [text_or_reparse!(substring, styling.reparse)]})
            }

            Formatting::Link(url) if styling.italic => {
                module!("link", url, { "label": escape_by_interspersion(substring) })
            }
            _ => text_or_reparse!(substring, styling.reparse),
        };
        values.push(value);
    }
    values.push(text_or_reparse!(
        &string.value[last_unformatted..],
        styling.reparse
    ));

    if citation_brackets {
        if let Some(rb) = styling.cit_style.brackets().right() {
            values.push(text!(rb));
        }
    }

    values
}

/// This function escapes a string that is going to be parsed by ModMark again by interspersing it
/// with backslashes (\). This means that the backslash escapes the next character, making it
/// impossible to have it be a part of a module, tag or smart punctuation
fn escape_by_interspersion(s: &str) -> String {
    let mut res = String::new();
    for c in s.chars() {
        res.push('\\');
        res.push(c);
    }
    res
}

/// This function returns its input with LaTeX comments removed (i.e, all lines starting with %
/// removed)
fn without_latex_comments(string: &str) -> String {
    string
        .lines()
        .filter(|line| !line.trim_start().starts_with('%'))
        .collect::<Vec<_>>()
        .join("\n")
}
