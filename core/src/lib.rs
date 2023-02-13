use std::str::FromStr;

use parser::{Element, ModuleArguments};

mod context;
mod error;
mod package;

pub use context::Context;
pub use error::CoreError;
pub use package::{ArgInfo, NodeName, Package, PackageInfo, Transform};

#[cfg(all(feature = "web", feature = "native"))]
compile_error!("feature \"native\" and feature \"web\" cannot be enabled at the same time");

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputFormat(String);

/// To ensure that "html" and "HTML" is the same.
impl OutputFormat {
    pub fn new(format: &str) -> Self {
        OutputFormat(format.to_lowercase())
    }
}

impl FromStr for OutputFormat {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(OutputFormat::new(s))
    }
}

/// Evaluates a document using the given context
pub fn eval(source: &str, ctx: &mut Context, format: &OutputFormat) -> Result<String, CoreError> {
    let document = parser::parse(source)?;
    eval_elem(document, ctx, format)
}

pub fn eval_elem(
    root: Element,
    ctx: &mut Context,
    format: &OutputFormat,
) -> Result<String, CoreError> {
    use Element::*;
    match root {
        Data(text) => {
            // FIXME: den här borde inte finnas, men kan så länge konvertera till en Module {name: "escape_text"}
            eval_elem(
                ModuleInvocation {
                    name: "escape_text".to_string(),
                    args: ModuleArguments {
                        positioned: None,
                        named: None,
                    },
                    body: text.clone(),
                    one_line: true,
                },
                ctx,
                format,
            )
        }
        Node {
            name: _,
            environment: _,
            children: _,
        } => {
            // skicka in allt till ctx.transform utan att evaluera barnen först, det får transformen göra bäst den vill med
            let compound = ctx.transform(&root, format)?;
            eval_elem(compound, ctx, format)
        }
        Compound(children) => {
            let mut raw_content = String::new();

            for child in children {
                raw_content.push_str(&eval_elem(child, ctx, format)?);
            }
            // FIXME: should add a Element::Raw variant.
            Ok(raw_content)
        }
        ModuleInvocation {
            ref name,
            args: _,
            ref body,
            one_line: _,
        } => {
            // Base case, if its just raw content, stop.
            if name == "raw" {
                return Ok(body.clone());
            }

            let compound = ctx.transform(&root, format)?;
            eval_elem(compound, ctx, format)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_info() {
        let ctx = Context::default();
        let info = ctx.get_package_info("Module test").unwrap().clone();

        let foo = PackageInfo {
            name: "Module test".to_string(),
            version: "1".to_string(),
            transforms: vec![
                Transform {
                    from: "[table]".to_string(),
                    to: vec![OutputFormat::new("table")],
                    args_info: vec![ArgInfo {
                        name: "border".to_string(),
                        default: Some("black".to_string()),
                        description: "What color the border should be".to_string(),
                    }],
                },
                Transform {
                    from: "table".to_string(),
                    to: vec![OutputFormat::new("html"), OutputFormat::new("latex")],
                    args_info: vec![],
                },
                Transform {
                    from: "row".to_string(),
                    to: vec![OutputFormat::new("html"), OutputFormat::new("latex")],
                    args_info: vec![],
                },
            ],
        };

        assert_eq!(info, foo);
    }
}
