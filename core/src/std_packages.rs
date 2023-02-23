use std::collections::HashMap;

use either::Either::{self, Left, Right};

use parser::ModuleArguments;

use crate::context::Issue;
use crate::std_packages_macros::{define_native_packages, define_standard_package_loader};
use crate::{ArgInfo, Context, CoreError, Element, OutputFormat, PackageInfo, Transform};

// Here, all standard packages are declared. The macro expands to one function
// which takes a &mut Context and loads it with the given standard package. It is important that the
// package with a given name both is in a folder with that name, containing a cargo package with
// that name. Otherwise, the module won't be found
define_standard_package_loader! {
    "table", "html",
}

// Here, all native packages are declared. The macro expands to two functions,
// one being a function returning the manifests for those packages, and the other
// being the entry point to run these packages.
define_native_packages! {
    "core" => {
        "raw", vec![] => native_raw,
        "warning", vec![
            ArgInfo {
                name: "source".to_string(),
                default: Some("<unknown>".to_string()),
                description: "The source module/parent responsible for the warning".to_string()
            },
            ArgInfo {
                name: "target".to_string(),
                default: Some("<unknown>".to_string()),
                description: "The target output format when the warning was generated".to_string()
            },
        ] => native_warn,
        "error", vec![
            ArgInfo {
                name: "source".to_string(),
                default: Some("<unknown>".to_string()),
                description: "The source module/parent responsible for the error".to_string()
            },
            ArgInfo {
                name: "target".to_string(),
                default: Some("<unknown>".to_string()),
                description: "The target output format when the error was generated".to_string()
            },
        ] => native_err
    };
    "reparse" => {
        "inline_content", vec![] => native_inline_content,
        "block_content", vec![] => native_block_content,
    };
    "env" => {
        "set-env", vec![
            ArgInfo {
                name: "key".to_string(),
                default: None,
                description: "The key to set".to_string()
            }
        ] => native_set_env,
    };
}

/// Returns a string containing the body of this invocation. This is the "leaf" call; no tree will
/// be created as a result of this call
pub fn native_raw(
    _ctx: &mut Context,
    body: &str,
    _args: HashMap<String, String>,
    _inline: bool,
    _output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    Ok(Right(body.to_owned()))
}

/// Re-parses the content as block content (a paragraph with tags, modules etc) and
/// returns the resulting compound element containing the contents of the paragraph
pub fn native_inline_content(
    _ctx: &mut Context,
    body: &str,
    _args: HashMap<String, String>,
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
pub fn native_block_content(
    _ctx: &mut Context,
    body: &str,
    _args: HashMap<String, String>,
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
pub fn native_set_env(
    _ctx: &mut Context,
    _body: &str,
    _args: HashMap<String, String>,
    _inline: bool,
    _output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    unimplemented!("native_set_env")
}

pub fn native_warn(
    ctx: &mut Context,
    body: &str,
    args: HashMap<String, String>,
    _inline: bool,
    _output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    let source = args.get("source").unwrap();
    let target = args.get("target").unwrap();

    // Push the issue to warnings
    ctx.state.warnings.push(Issue {
        source: source.to_string(),
        target: target.to_string(),
        description: body.to_string(),
    });

    // Return no new nodes
    Ok(Left(Element::Compound(vec![])))
}

pub fn native_err(
    ctx: &mut Context,
    body: &str,
    args: HashMap<String, String>,
    inline: bool,
    output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    let source = args.get("source").unwrap();
    let target = args.get("target").unwrap();

    // Push the issue to errors
    ctx.state.errors.push(Issue {
        source: source.to_string(),
        target: target.to_string(),
        description: body.to_string(),
    });

    // Check if we have an __error transform
    if !ctx
        .transforms
        .get("__error")
        .and_then(|t| t.find_transform_to(output_format))
        .is_some()
        || source == "__error"
    {
        // If we don't have, don't add an __error parent since that would yield an CoreError
        // Also if the error did originate from an error module itself, don't generate more and
        // crash
        Ok(Left(Element::Compound(vec![])))
    } else {
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
