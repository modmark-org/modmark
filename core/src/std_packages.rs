use std::collections::HashMap;

use crate::context::Either;
use crate::context::Either::{Left, Right};
use crate::std_packages_macros::{define_native_packages, define_standard_package_loader};
use crate::{ArgInfo, Context, CoreError, Element, OutputFormat, PackageInfo, Transform};

// Here, all native packages are declared. The macro expands to two functions,
// one being a function returning the manifests for those packages, and the other
// being the entry point to run these packages.
define_native_packages! {
    "core" => {
        "raw", vec![] => native_raw,
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

// Here, all standard packages are declared. The macro expands to one function
// which takes a &mut Context and loads it with the given standard package. It is important that the
// package with a given name both is in a folder with that name, containing a cargo package with
// that name. Otherwise, the module won't be found
define_standard_package_loader! {
    "table", "html",
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
