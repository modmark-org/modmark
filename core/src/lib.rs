use std::error::Error;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::str::FromStr;

use either::Either::{Left, Right};
use serde::{Deserialize, Serialize};

pub use context::Context;
pub use element::Element;
pub use error::CoreError;
pub use package::{ArgInfo, Package, PackageInfo, Transform};
use package_manager::Resolve;

use crate::context::CompilationState;

pub mod context;
mod element;
mod error;
mod fs;
mod package;
pub mod package_manager;
mod std_packages;
mod std_packages_macros;
#[cfg(all(feature = "web", feature = "native"))]
compile_error!("feature \"native\" and feature \"web\" cannot be enabled at the same time");


pub trait AccessPolicy: Send + Sync + 'static {
    fn root(&self) -> Option<String>;
    fn allowed_to_read(&self) -> bool;
    fn allowed_to_write(&self) -> bool;
    fn allowed_to_create(&self) -> bool;
    fn allowed_access(&mut self, path: &Path, module_name: &String) -> bool;
}

pub struct DefaultAccessManager;

impl AccessPolicy for DefaultAccessManager {
    fn root(&self) -> Option<String> {
        Some(String::from("/"))
    }

    fn allowed_to_read(&self) -> bool {
        true
    }

    fn allowed_to_write(&self) -> bool {
        true
    }

    fn allowed_to_create(&self) -> bool {
        true
    }

    fn allowed_access(&mut self, _path: &Path, _module_name: &String) -> bool {
        true
    }
}

#[derive(Debug, Clone, Eq, Deserialize, Serialize)]
pub struct OutputFormat(String);

impl OutputFormat {
    pub fn new(string: &str) -> Self {
        OutputFormat(string.to_lowercase())
    }
}

/// To ensure that "html" and "HTML" is the same.
impl PartialEq for OutputFormat {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

impl Hash for OutputFormat {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_lowercase().hash(state);
    }
}

impl ToString for OutputFormat {
    fn to_string(&self) -> String {
        self.0.to_lowercase()
    }
}

impl FromStr for OutputFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(OutputFormat::new(s))
    }
}

/// Evaluates a document using the given context
pub fn eval<T, U>(
    source: &str,
    ctx: &mut Context<T, U>,
    format: &OutputFormat,
) -> Result<Option<(String, CompilationState)>, CoreError>
where
    T: Resolve,
    U: AccessPolicy + Send + Sync + 'static,
{
    // Note: this isn't actually needed, since take_state clears state, but it
    // is still called to ensure that it is cleared, if someone uses any context mutating functions
    // outside of here which doesn't take state afterwards
    ctx.clear_state();

    // TODO: Move this out so that we have a flag in the CLI and a switch in the playground to
    //   do verbose errors or "debug mode" or similar
    ctx.state.verbose_errors = true;
    let (doc_ast, config) = parser::parse_with_config(source)?;
    let document = doc_ast.try_into()?;
    let success = ctx.configure(config)?;
    if !success {
        println!("No success");
        return Ok(None);
    }

    let res = eval_elem(document, ctx, format);

    res.map(|s| Some((s, ctx.take_state())))
}

/// Evaluates a document using the given context without a document element
pub fn eval_no_document<T, U>(
    source: &str,
    ctx: &mut Context<T, U>,
    format: &OutputFormat,
) -> Result<Option<(String, CompilationState)>, CoreError>
where
    T: Resolve,
    U: AccessPolicy + Send + Sync + 'static,
{
    ctx.clear_state();

    // TODO: Move this out so that we have a flag in the CLI and a switch in the playground to
    //   do verbose errors or "debug mode" or similar
    ctx.state.verbose_errors = true;
    let (doc_ast, config) = parser::parse_with_config(source)?;
    let document: Element = doc_ast.try_into()?;
    let no_doc = if let Element::Parent {
        name: _,
        args: _,
        children,
    } = document
    {
        Ok(Element::Compound(children))
    } else {
        Err(CoreError::RootElementNotParent)
    }?;

    let success = ctx.configure(config)?;
    if !success {
        return Ok(None);
    }

    let res = eval_elem(no_doc, ctx, format);

    res.map(|s| Some((s, ctx.take_state())))
}

pub fn eval_elem<T, U>(
    root: Element,
    ctx: &mut Context<T, U>,
    format: &OutputFormat,
) -> Result<String, CoreError>
where
    U: AccessPolicy + Send + Sync + 'static,
{
    use Element::{Compound, Module, Parent};
    match root {
        Compound(children) => {
            let mut raw_content = String::new();

            for child in children {
                raw_content.push_str(&eval_elem(child, ctx, format)?);
            }
            Ok(raw_content)
        }
        Module {
            name: _,
            args: _,
            body: _,
            inline: _,
        }
        | Parent {
            name: _,
            args: _,
            children: _,
        } => {
            let either = ctx.transform(&root, format)?;
            match either {
                Left(elem) => eval_elem(elem, ctx, format),
                Right(res) => Ok(res),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::package::{ArgType, PrimitiveArgType};

    use super::*;

    #[test]
    fn table_manifest_test() {
        let ctx = Context::new_without_resolver().unwrap();
        let info = ctx.get_package_info("table").unwrap().clone();

        let foo = PackageInfo {
            name: "table".to_string(),
            version: "0.1".to_string(),
            description: "This package supports [table] modules".to_string(),
            transforms: vec![Transform {
                from: "table".to_string(),
                to: vec![OutputFormat::new("html"), OutputFormat::new("latex")],
                description: None,
                arguments: vec![
                    ArgInfo {
                        name: "caption".to_string(),
                        default: Some(Value::String("".to_string())),
                        description: "The caption for the table".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    }, ArgInfo {
                        name: "label".to_string(),
                        default: Some(Value::String("".to_string())),
                        description: "The label to use for the table, to be able to refer to it from the document".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    }, ArgInfo {
                        name: "header".to_string(),
                        default: Some(Value::String("none".to_string())),
                        description: "Style to apply to heading, none/bold".to_string(),
                        r#type: ArgType::Enum(vec!["none".to_string(), "bold".to_string()])
                    }, ArgInfo {
                        name: "alignment".to_string(),
                        default: Some(Value::String("left".to_string())),
                        description: "Horizontal alignment in cells, left/center/right or l/c/r for each column".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    }, ArgInfo {
                        name: "borders".to_string(),
                        default: Some(Value::String("all".to_string())),
                        description: "Which borders to draw".to_string(),
                        r#type: ArgType::Enum(vec!["all".to_string(), "horizontal".to_string(), "vertical".to_string(), "outer".to_string(), "none".to_string()])
                    }, ArgInfo {
                        name: "delimiter".to_string(),
                        default: Some(Value::String("|".to_string())),
                        description: "The delimiter between cells".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    }, ArgInfo {
                        name: "strip_whitespace".to_string(),
                        default: Some(Value::String("true".to_string())),
                        description: "true/false to strip/don't strip whitespace in cells".to_string(),
                        r#type: ArgType::Enum(vec!["true".to_string(), "false".to_string()])
                    },
                ],
            }],
        };

        assert_eq!(info, foo);
    }
}
