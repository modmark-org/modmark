use std::hash::{Hash, Hasher};
use std::str::FromStr;

use either::Either::{Left, Right};
use serde::{Deserialize, Serialize};

pub use context::Context;
pub use element::Element;
pub use error::CoreError;
pub use package::{ArgInfo, Package, PackageInfo, Transform};

use crate::context::CompilationState;

pub mod context;
mod element;
mod error;
mod package;
mod std_packages;
mod std_packages_macros;

#[cfg(all(feature = "web", feature = "native"))]
compile_error!("feature \"native\" and feature \"web\" cannot be enabled at the same time");

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
pub fn eval(
    source: &str,
    ctx: &mut Context,
    format: &OutputFormat,
) -> Result<(String, CompilationState), CoreError> {
    // Note: this isn't actually needed, since take_state clears state, but it
    // is still called to ensure that it is cleared, if someone uses any context mutating functions
    // outside of here which doesn't take state afterwards
    ctx.clear_state();

    // TODO: Move this out so that we have a flag in the CLI and a switch in the playground to
    //   do verbose errors or "debug mode" or similar
    ctx.state.verbose_errors = true;
    let document = parser::parse(source)?.try_into()?;
    let res = eval_elem(document, ctx, format);

    res.map(|s| (s, ctx.take_state()))
}

pub fn eval_elem(
    root: Element,
    ctx: &mut Context,
    format: &OutputFormat,
) -> Result<String, CoreError> {
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
    use super::*;

    #[test]
    fn table_manifest_test() {
        let ctx = Context::default();
        let info = ctx.get_package_info("table").unwrap().clone();

        let foo = PackageInfo {
            name: "table".to_string(),
            version: "0.1".to_string(),
            description: "This package supports [table] modules".to_string(),
            transforms: vec![Transform {
                from: "table".to_string(),
                to: vec![OutputFormat::new("html"), OutputFormat::new("latex")],
                arguments: vec![
                    ArgInfo {
                        name: "header".to_string(),
                        default: Some("none".to_string()),
                        description: "Style to apply to heading, none/bold".to_string(),
                    }, ArgInfo {
                        name: "alignment".to_string(),
                        default: Some("left".to_string()),
                        description: "Horizontal alignment in cells, left/center/right or l/c/r for each column".to_string(),
                    }, ArgInfo {
                        name: "borders".to_string(),
                        default: Some("all".to_string()),
                        description: "Which borders to draw, all/horizontal/vertical/outer/none".to_string(),
                    }, ArgInfo {
                        name: "delimiter".to_string(),
                        default: Some("|".to_string()),
                        description: "The delimiter between cells".to_string(),
                    }, ArgInfo {
                        name: "strip_whitespace".to_string(),
                        default: Some("true".to_string()),
                        description: "true/false to strip/don't strip whitespace in cells".to_string(),
                    },
                ],
            }],
        };

        assert_eq!(info, foo);
    }
}
