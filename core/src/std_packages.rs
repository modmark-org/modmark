use std::collections::HashMap;

use crate::context::Either;
use crate::context::Either::Right;
use crate::std_packages_macros::{define_native_packages, define_standard_package_loader};
use crate::{ArgInfo, Context, CoreError, Element, OutputFormat, PackageInfo, Transform};

pub struct StandardPackages {}

define_native_packages! {
    StandardPackages;
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
        ] => native_inline_content,
    };
}

define_standard_package_loader! {
    StandardPackages;
    "table", "html",
}

/*pub fn handle_native_invocation(
    ctx: &mut Context,
    package_name: &String,
    node_name: &String, // name of module or parent
    element: &Element,
    args: HashMap<String, String>,
) -> Result<Element, CoreError> {
    match package_name.into() {
        "parser" => match node_name.into() {
            "parse_inline" => match element {
                Element::Module {
                    name,
                    args: _,
                    body,
                    inline,
                } => native_inline_content(ctx, body, args, *inline),
                _ => unimplemented!(),
            },
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
    unimplemented!()
}*/

pub fn native_raw(
    ctx: &mut Context,
    body: &String,
    args: HashMap<String, String>,
    inline: bool,
    output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    Ok(Right(body.clone()))
}

pub fn native_inline_content(
    ctx: &mut Context,
    body: &String,
    args: HashMap<String, String>,
    inline: bool,
    output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    let elements = parser::parse_inline(body)?
        .into_iter()
        .map(|ast| ast.try_into())
        .collect::<Result<Vec<Element>, _>>()?;

    return Ok(Right(crate::eval_elem(
        Element::Compound(elements),
        ctx,
        output_format,
    )?));
}

pub fn native_block_content(
    ctx: &mut Context,
    body: &String,
    args: HashMap<String, String>,
    inline: bool,
    output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    let elements = parser::parse_blocks(body)?
        .into_iter()
        .map(|ast| ast.try_into())
        .collect::<Result<Vec<Element>, _>>()?;

    Ok(Right(crate::eval_elem(
        Element::Compound(elements),
        ctx,
        output_format,
    )?))
}

pub fn native_set_env(
    ctx: &mut Context,
    body: &String,
    args: HashMap<String, String>,
    inline: bool,
    output_format: &OutputFormat,
) -> Result<Either<Element, String>, CoreError> {
    unimplemented!("native_set_env")
}
