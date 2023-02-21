use std::collections::HashMap;

use crate::context::Either;
use crate::context::Either::Right;
use crate::std_packages_macros::{define_native_packages, define_standard_package_loader};
use crate::{ArgInfo, Context, CoreError, Element, OutputFormat, PackageInfo, Transform};

// Here, all native packages are declared. The macro expands to two functions,
// one being a function returning the manifests for those packages, and the other
// being the entry point to run these packages
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

/*pub fn handle_native_invocation(
    ctx: &mut Context,
    package_name: &String,
    node_name: &String, // name of module or parent
    element: &Element,
    args: HashMap<String, String>,
) -> Result<Element, CoreError> {
}*/

pub fn native_raw(
    _ctx: &mut Context,
    body: &str,
    _args: HashMap<String, String>,
    _inline: bool,
    _output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    Ok(Right(body.to_owned()))
}

pub fn native_inline_content(
    ctx: &mut Context,
    body: &str,
    _args: HashMap<String, String>,
    _inline: bool,
    output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    let elements = parser::parse_inline(body)?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<Element>, _>>()?;

    Ok(Right(crate::eval_elem(
        Element::Compound(elements),
        ctx,
        output_format,
    )?))
}

pub fn native_block_content(
    ctx: &mut Context,
    body: &str,
    _args: HashMap<String, String>,
    _inline: bool,
    output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    let elements = parser::parse_blocks(body)?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<Element>, _>>()?;

    Ok(Right(crate::eval_elem(
        Element::Compound(elements),
        ctx,
        output_format,
    )?))
}

pub fn native_set_env(
    _ctx: &mut Context,
    _body: &str,
    _args: HashMap<String, String>,
    _inline: bool,
    _output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    unimplemented!("native_set_env")
}
