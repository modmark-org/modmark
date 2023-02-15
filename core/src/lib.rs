use std::str::FromStr;

use parser::{Element, ModuleArguments};

mod context;
mod error;
mod package;

pub use context::Context;
pub use error::CoreError;
pub use package::{ArgInfo, NodeName, Package, PackageInfo, Transform};
use serde::Deserialize;
use std::hash::{Hash, Hasher};

#[cfg(all(feature = "web", feature = "native"))]
compile_error!("feature \"native\" and feature \"web\" cannot be enabled at the same time");

#[derive(Debug, Clone, Eq, Deserialize)]
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

impl FromStr for OutputFormat {
    //FIXME this does not work when i do cargo test, might have to refactor
    // type Err = core::convert::Infallible;
    type Err = ();

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
    fn table_manifest_test() {
        let ctx = Context::default();
        let info = ctx
            .get_package_info("Standard table module")
            .unwrap()
            .clone();

        let foo = PackageInfo {
            name: "Standard table module".to_string(),
            version: "0.1".to_string(),
            description: "This package supports [table] modules".to_string(),
            transforms: vec![Transform {
                from: "table".to_string(),
                to: vec![OutputFormat::new("html")],
                arguments: vec![ArgInfo {
                    name: "col_delimiter".to_string(),
                    default: Some("|".to_string()),
                    description: "The string delimiter for columns".to_string(),
                }],
            }],
        };

        assert_eq!(info, foo);
    }
}
