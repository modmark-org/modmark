use std::collections::HashMap;

use serde_json::Value;
#[cfg(feature = "native")]
use wasmer::Engine;

use parser::ModuleArguments;

use crate::context::Issue;
use crate::element::GranularId;
use crate::package::{ArgValue, PrimitiveArgType};
use crate::package_store::PackageStore;
use crate::std_packages_macros::{define_native_packages, define_standard_package_loader};
use crate::variables::{ListAccess, VarAccess};
use crate::{ArgInfo, Context, CoreError, Element, OutputFormat, PackageInfo, Transform};

// Here, all standard packages are declared. The macro expands to one function
// which takes a &mut Context and loads it with the given standard package. It is important that the
// package with a given name both is in a folder with that name, containing a cargo package with
// that name. Otherwise, the module won't be found
define_standard_package_loader! {
    "table",
    "html",
    "latex",
    "link",
    "list",
    "code",
    "math",
    "layout",
    "files",
    "flow",
}

// Here, all native packages are declared. The macro expands to two functions,
// one being a function returning the manifests for those packages, and the other
// being the entry point to run these packages.
define_native_packages! {
    "core" => {
        desc: "Provides core functionality such as raw output, errors and warnings",
        transforms: [
            {
                name: "raw",
                desc: "Outputs the body text as-is into the output document",
                vars: [],
                args: vec![],
                func: native_raw
            },
            {
                name: "warning",
                desc: "Adds a compilation warning to the list of all warnings that have occurred during compilation",
                vars: [],
                args: vec![
                    ArgInfo {
                        name: "source".to_string(),
                        default: Some(Value::String("<unknown>".to_string())),
                        description: "The source module/parent responsible for the warning".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                    ArgInfo {
                        name: "target".to_string(),
                        default: Some(Value::String("<unknown>".to_string())),
                        description: "The target output format when the warning was generated".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                    ArgInfo {
                        name: "input".to_string(),
                        default: Some(Value::String("<unknown>".to_string())),
                        description: "The input given to the module when it failed".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                ],
                func: native_warn
            },
            {
                name: "error",
                desc: "Adds a compilation error to the list of all errors that have occurred during compilation",
                vars: [],
                args: vec![
                    ArgInfo {
                        name: "source".to_string(),
                        default: Some(Value::String("<unknown>".to_string())),
                        description: "The source module/parent responsible for the error".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                    ArgInfo {
                        name: "target".to_string(),
                        default: Some(Value::String("<unknown>".to_string())),
                        description: "The target output format when the error was generated".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                    ArgInfo {
                        name: "input".to_string(),
                        default: Some(Value::String("<unknown>".to_string())),
                        description: "The input given to the module when it failed".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                ],
                func: native_err
            }
        ]
    }
    "reparse" => {
        desc: "Provides an interface to the built-in ModMark parser",
        transforms: [
            {
                name: "inline_content",
                desc: "Parses the content as inline-content, as if it was in a paragraph. The result may contain text, smart punctuation, inline module expressions and tags",
                vars: [],
                args: vec![],
                func: native_inline_content
            },
            {
                name: "block_content",
                desc: "Parses the content as block-content, as if it was in the body of the document. The result may contain paragraphs containing inline content and multiline module expressions",
                vars: [],
                args: vec![],
                func: native_block_content
            }
        ]
    }
    "env" => {
        desc: "[Temporary] Provides access to setting environment variables",
        transforms: [
            {
                name: "push-list",
                desc: "Pushes a string to a list",
                vars: [("$key".to_string(), VarAccess::List(ListAccess::Push))],
                args: vec![
                    ArgInfo {
                        name: "key".to_string(),
                        default: None,
                        description: "The key to set".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    }
                ],
                func: native_push_list
            },
            {
                name: "read-list",
                desc: "Reads all strings from a list",
                vars: [("$key".to_string(), VarAccess::List(ListAccess::Read))],
                args: vec![
                    ArgInfo {
                        name: "key".to_string(),
                        default: None,
                        description: "The key to set".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    }
                ],
                func: native_read_list
            }
        ]
    }
}

/// Returns a string containing the body of this invocation. This is the "leaf" call; no tree will
/// be created as a result of this call
pub fn native_raw<T, U>(
    _ctx: &mut Context<T, U>,
    body: &str,
    _args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
    _id: &GranularId,
) -> Result<Element, CoreError> {
    Ok(Element::Raw(body.to_owned()))
}

/// Re-parses the content as block content (a paragraph with tags, modules etc) and
/// returns the resulting compound element containing the contents of the paragraph
pub fn native_inline_content<T, U>(
    _ctx: &mut Context<T, U>,
    body: &str,
    _args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
    id: &GranularId,
) -> Result<Element, CoreError> {
    let elements = parser::parse_inline(body)?
        .into_iter()
        .zip(id.children())
        .map(|(ast, id)| Element::try_from_ast(ast, id))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Element::Compound(elements))
}

/// Re-parses the content as block content (multiple paragraph or multiline module invocations) and
/// returns the resulting compound element
pub fn native_block_content<T, U>(
    _ctx: &mut Context<T, U>,
    body: &str,
    _args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
    id: &GranularId,
) -> Result<Element, CoreError> {
    let elements = parser::parse_blocks(body)?
        .into_iter()
        .zip(id.children())
        .map(|(ast, id)| Element::try_from_ast(ast, id))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Element::Compound(elements))
}

/// Example function for pushing a string to a list
pub fn native_push_list<T, U>(
    ctx: &mut Context<T, U>,
    body: &str,
    args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
    _id: &GranularId,
) -> Result<Element, CoreError> {
    let key = args.get("key").unwrap().as_str().unwrap();
    let entry = ctx.lists.entry(key.to_string()).or_default();
    entry.push(body.to_string());
    Ok(Element::Compound(vec![]))
}

/// Example function for reading a list
pub fn native_read_list<T, U>(
    ctx: &mut Context<T, U>,
    _body: &str,
    args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
    id: &GranularId,
) -> Result<Element, CoreError> {
    let key = args.get("key").unwrap().as_str().unwrap();
    if let Some(list) = ctx.lists.get(key) {
        Ok(Element::Module {
            name: "__text".to_string(),
            args: Default::default(),
            body: list.join(","),
            inline: false,
            id: id.clone(),
        })
    } else {
        Ok(Element::Compound(vec![]))
    }
}

pub fn native_warn<T, U>(
    ctx: &mut Context<T, U>,
    body: &str,
    mut args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
    _id: &GranularId,
) -> Result<Element, CoreError> {
    // Push the issue to warnings
    ctx.state.warnings.push(Issue {
        source: args.remove("source").unwrap().get_string().unwrap(),
        target: args.remove("target").unwrap().get_string().unwrap(),
        description: body.to_string(),
        input: args
            .remove("input")
            .map(|v| v.get_string().unwrap())
            .and_then(|s| (s != "<unknown>").then_some(s)),
    });

    // Return no new nodes
    Ok(Element::Compound(vec![]))
}

pub fn native_err<T, U>(
    ctx: &mut Context<T, U>,
    body: &str,
    args: HashMap<String, ArgValue>,
    inline: bool,
    output_format: &OutputFormat,
    id: &GranularId,
) -> Result<Element, CoreError> {
    let source = args.get("source").unwrap().clone().get_string().unwrap();
    let target = args.get("target").unwrap().clone().get_string().unwrap();
    let input = args
        .get("input")
        .map(|v| v.clone().get_string().unwrap())
        .and_then(|s| (s != "<unknown>").then_some(s));

    // Push the issue to errors
    ctx.state.errors.push(Issue {
        source: source.to_string(),
        target,
        description: body.to_string(),
        input,
    });

    // Check if we have an __error transform
    if ctx
        .package_store
        .lock()
        .unwrap()
        .transforms
        .get("__error")
        .and_then(|t| t.find_transform_to(output_format))
        .is_none()
        || source == "__error"
    {
        // If we don't have, don't add an __error parent since that would yield an CoreError
        // Also if the error did originate from an error module itself, don't generate more and
        // crash
        Ok(Element::Compound(vec![]))
    } else {
        let type_erase = |mut map: HashMap<String, ArgValue>| {
            map.drain()
                .map(|(k, v)| (k, v.into()))
                .collect::<HashMap<String, String>>()
        };

        let args = type_erase(args);

        // If we do, add an __error parent since our output format should
        Ok(Element::Module {
            name: "__error".to_string(),
            body: body.to_string(),
            args: ModuleArguments {
                positioned: None,
                named: Some(args),
            },
            inline,
            id: id.clone(),
        })
    }
}
