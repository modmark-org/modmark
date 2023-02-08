use std::collections::HashMap;
use std::fs;
use std::path::Path;

use json::{object, JsonValue};

use parser::{parse, parse_to_ast_document, Ast, Document, Element};
use diffy::{create_patch, merge};

fn split_test(input: &Path) -> datatest_stable::Result<()> {
    let output = input.with_extension("json");

    // We want to test the input one time with LF line endings and one time with CRLF endings.
    // For this, we first need to ensure that the input file ends with \n and not \r\n (normalize
    // it).
    // For the output file, we don't actually need to care about the line endings, but we want to
    // care about
    let input_data = fs::read_to_string(input).unwrap().replace("\r\n", "\n");
    let output_data = fs::read_to_string(&output)
        .unwrap_or_else(|_| {
            panic!(
                "Input file should have matching output file {}",
                output.display()
            )
        })
        .replace(r"\r\n", r"\n");

    test_lf(&input_data, &output_data);
    test_crlf(&input_data, &output_data);

    Ok(())
}

fn unified_test(input: &Path) -> datatest_stable::Result<()> {
    let inp = fs::read_to_string(input).unwrap();
    let lines = inp.lines().collect::<Vec<&str>>();
    let blocks = lines
        .split(|l| l.starts_with("```"))
        .enumerate()
        .filter_map(|(idx, l)| (idx % 2 == 1).then_some(l))
        .collect::<Vec<&[&str]>>();

    let input = blocks.get(0).unwrap().join("\n");
    let output = blocks.get(1).unwrap().join("\n");

    test_lf(&input, &output);
    test_crlf(&input, &output);
    Ok(())
}

fn test_lf(input: &str, output: &str) {
    let mdm_obj = elem_to_json(&parse(input));
    let ast_obj = doc_to_json(parse_to_ast_document(input));
    let json_obj = json::parse(output).expect("JSON should be parsable");

    // note: we DO NOT want assert_eq here since that would print the mismatched
    // json IR:s, but the custom error message is much easier to read
    if mdm_obj != json_obj {
        panic!(
            "Failed using LF,\nEXPECTED\n{}\nGOT\n{}\nDIFF\n{}",
            json_obj.pretty(2),
            mdm_obj.pretty(2),
            create_patch(&json_obj.pretty(2), &mdm_obj.pretty(2)).to_string()
        );
    }

    if ast_obj != json_obj {
        panic!(
            "Failed using LF,\nEXPECTED\n{}\nGOT\n{}\nDIFF\n{}",
            json_obj.pretty(2),
            ast_obj.pretty(2),
            create_patch(&json_obj.pretty(2), &ast_obj.pretty(2)).to_string()
        );
    }
}

fn test_crlf(input: &str, output: &str) {
    let mdm_obj = elem_to_json(&parse(&input.replace('\n', "\r\n")));
    let ast_obj = doc_to_json(parse_to_ast_document(&input.replace('\n', "\r\n")));
    let json_obj = json::parse(&output.replace(r"\n", r"\r\n")).expect("JSON should be parsable");

    // note: we DO NOT want assert_eq here since that would print the mismatched
    // json IR:s, but the custom error message is much easier to read
    if mdm_obj != json_obj {
        panic!(
            "Failed using CRLF,\nEXPECTED\n{}\nGOT\n{}\nDIFF\n{}",
            json_obj.pretty(2),
            mdm_obj.pretty(2),
            create_patch(&json_obj.pretty(2), &mdm_obj.pretty(2)).to_string()
        );
    }

    if ast_obj != json_obj {
        panic!(
            "Failed using CRLF,\nEXPECTED\n{}\nGOT\n{}\nDIFF\n{}",
            json_obj.pretty(2),
            ast_obj.pretty(2),
            create_patch(&json_obj.pretty(2), &ast_obj.pretty(2)).to_string()
        );
    }
}

fn doc_to_json(doc: Document) -> JsonValue {
    ast_to_json(&Ast::Document(doc))
}

fn ast_to_json(ast: &Ast) -> JsonValue {
    match ast {
        Ast::Text(str) => str.as_str().into(),
        Ast::Document(d) => {
            object! {
                name: "Document",
                children: d.elements.iter().map(ast_to_json).collect::<Vec<JsonValue>>()
            }
        }
        Ast::Paragraph(p) => {
            object! {
                name: "Paragraph",
                children: p.elements.iter().map(ast_to_json).collect::<Vec<JsonValue>>()
            }
        }
        Ast::Tag(t) => {
            object! {
                name: t.tag_name.as_str(),
                children: t.elements.iter().map(ast_to_json).collect::<Vec<JsonValue>>()
            }
        }
        Ast::Module(m) => {
            object! {
                name: m.name.as_str(),
                args: JsonValue::from(m.args.positioned.clone().unwrap_or_default().iter().enumerate().map(|(a,b)| (a.to_string(),b.to_string())).chain(
                    m.args.named.clone().unwrap_or_default().iter().map(|(a,b)| (a.to_string(), b.to_string()))
                ).collect::<HashMap<String, String>>()),
                body: m.body.as_str(),
                one_line: JsonValue::from(m.one_line),
            }
        }
    }
}

// this is used to compare the resulting parsed Element to the expected Json
// since the Ast JSON representation is compared as well, if Element is changed
// too much, this may possibly be removed
fn elem_to_json(elem: &Element) -> JsonValue {
    match elem {
        Element::Data(str) => str.as_str().into(),
        Element::Node {
            name,
            environment: _environment,
            children,
        } => {
            object! {
                name: name.as_str(),
                children: children.iter().map(elem_to_json).collect::<Vec<JsonValue>>()
            }
        }
        Element::ModuleInvocation {
            name,
            args,
            body,
            one_line,
        } => {
            object! {
                name: name.as_str(),
                args: JsonValue::from(args.positioned.clone().unwrap_or_default().iter().enumerate().map(|(a,b)| (a.to_string(),b.to_string())).chain(
                    args.named.clone().unwrap_or_default().iter().map(|(a,b)| (a.to_string(), b.to_string()))
                ).collect::<HashMap<String, String>>()),
                body: body.as_str(),
                one_line: JsonValue::from(*one_line),
            }
        }
    }
}

datatest_stable::harness!(
    split_test,
    "tests/compilation_tests",
    r"^.*.mdm$",
    unified_test,
    "tests/compilation_tests",
    r"^.*.mdmtest$"
);
