use std::collections::HashMap;

use either::Either::{self, Left, Right};
use serde_json::Value;
#[cfg(feature = "native")]
use wasmer::Engine;

use parser::ModuleArguments;

use crate::context::Issue;
use crate::package::{ArgValue, PrimitiveArgType};
use crate::package_store::PackageStore;
use crate::std_packages_macros::{define_native_packages, define_standard_package_loader};
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
}

// Here, all native packages are declared. The macro expands to two functions,
// one being a function returning the manifests for those packages, and the other
// being the entry point to run these packages.
define_native_packages! {
    "core",
    "Provides core functionality such as raw output, errors and warnings" => {
        "if",
        "Conditionally include content based on the output format",
        vec![ArgInfo {
            name: "format".to_string(),
            default: Some(Value::String("".to_string())),
            description: "The output format".to_string(),
            r#type: PrimitiveArgType::String.into()
        }] => native_if,
        "raw",
        "Outputs the body text as-is into the output document",
        vec![] => native_raw,
        "warning",
        "Adds a compilation warning to the list of all warnings that have occurred during compilation",
        vec![
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
        ] => native_warn,
        "error",
        "Adds a compilation error to the list of all errors that have occurred during compilation",
        vec![
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
        ] => native_err
    };
    "reparse",
    "Provides an interface to the built-in ModMark parser" => {
        "inline_content",
        "Parses the content as inline-content, as if it was in a paragraph. The result may contain text, smart punctuation, inline module expressions and tags",
        vec![] => native_inline_content,
        "block_content",
        "Parses the content as block-content, as if it was in the body of the document. The result may contain paragraphs containing inline content and multiline module expressions",
        vec![] => native_block_content,
    };
    "env",
    "[Temporary] Provides access to setting environment variables" => {
        "set-env",
        "Sets an environment variable",
        vec![
            ArgInfo {
                name: "key".to_string(),
                default: None,
                description: "The key to set".to_string(),
                r#type: PrimitiveArgType::String.into()
            }
        ] => native_set_env,
    };
}

/// Returns a string containing the body of this invocation. This is the "leaf" call; no tree will
/// be created as a result of this call
pub fn native_raw<T, U>(
    _ctx: &mut Context<T, U>,
    body: &str,
    _args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    Ok(Right(body.to_owned()))
}

/// ifdef style conditional compilation based on the output format
/// FIXME: move this into a normal standard module once we support the any output type
/// see issue GH-128.
pub fn native_if<T, U>(
    _ctx: &mut Context<T, U>,
    body: &str,
    args: HashMap<String, ArgValue>,
    inline: bool,
    output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    let cmp_format = args.get("format");

    if let Some(cmp_format) = cmp_format {
        let cmp_format = cmp_format.as_str().unwrap_or("");

        if !cmp_format.is_empty() && &OutputFormat::new(cmp_format) != output_format {
            // Maybe we could return an empty Element::Compund instead?
            // revist after the evalutor rewrite
            return Ok(Right("".to_string()));
        }
    }

    let parser = if inline {
        "inline_content"
    } else {
        "block_content"
    };

    Ok(Left(Element::Module {
        name: parser.to_string(),
        args: ModuleArguments {
            positioned: None,
            named: None,
        },
        body: body.to_string(),
        inline,
    }))
}

/// Re-parses the content as block content (a paragraph with tags, modules etc) and
/// returns the resulting compound element containing the contents of the paragraph
pub fn native_inline_content<T, U>(
    _ctx: &mut Context<T, U>,
    body: &str,
    _args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    let elements = parser::parse_inline(body)?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<Element>, _>>()?;

    Ok(Left(Element::Compound(elements)))
}

/// Re-parses the content as block content (multiple paragraph or multiline module invocations) and
/// returns the resulting compound element
pub fn native_block_content<T, U>(
    _ctx: &mut Context<T, U>,
    body: &str,
    _args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    let elements = parser::parse_blocks(body)?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<Element>, _>>()?;

    Ok(Left(Element::Compound(elements)))
}

/// Example function for setting environment variables, currently unimplemented
pub fn native_set_env<T, U>(
    _ctx: &mut Context<T, U>,
    _body: &str,
    _args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    unimplemented!("native_set_env")
}

pub fn native_warn<T, U>(
    ctx: &mut Context<T, U>,
    body: &str,
    mut args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
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
    Ok(Left(Element::Compound(vec![])))
}

pub fn native_err<T, U>(
    ctx: &mut Context<T, U>,
    body: &str,
    args: HashMap<String, ArgValue>,
    inline: bool,
    output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
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
        Ok(Left(Element::Compound(vec![])))
    } else {
        let type_erase = |mut map: HashMap<String, ArgValue>| {
            map.drain()
                .map(|(k, v)| (k, v.into()))
                .collect::<HashMap<String, String>>()
        };

        let args = type_erase(args);

        // If we do, add an __error parent since our output format should
        Ok(Left(Element::Module {
            name: "__error".to_string(),
            body: body.to_string(),
            args: ModuleArguments {
                positioned: None,
                named: Some(args),
            },
            inline,
        }))
    }
}
