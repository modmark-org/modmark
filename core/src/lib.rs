use parser::Element;

mod context;
mod error;
mod package;

pub use context::Context;
pub use error::CoreError;
pub use package::{Arg, NodeName, Package, PackageInfo, Transform};

#[cfg(all(feature = "web", feature = "native"))]
compile_error!("feature \"native\" and feature \"web\" cannot be enabled at the same time");

/// Evaluates a document using the given context
pub fn eval(_document: &Element, _ctx: &mut Context) -> String {
    "TODO".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_info() {
        let ctx = Context::default();
        let info = ctx.get_module_info("Module test").unwrap().clone();

        let foo = ModuleInfo {
            name: "Module test".to_string(),
            version: "1".to_string(),
            transforms: vec![
                Transform {
                    from: "[table]".to_string(),
                    to: "table".to_string(),
                    arguments: vec![Arg {
                        name: "border".to_string(),
                        default: Some("black".to_string()),
                        description: "What color the border should be".to_string(),
                    }],
                },
                Transform {
                    from: "table".to_string(),
                    to: "html".to_string(),
                    arguments: vec![],
                },
                Transform {
                    from: "table".to_string(),
                    to: "latex".to_string(),
                    arguments: vec![],
                },
                Transform {
                    from: "row".to_string(),
                    to: "html".to_string(),
                    arguments: vec![],
                },
                Transform {
                    from: "row".to_string(),
                    to: "latex".to_string(),
                    arguments: vec![],
                },
            ],
        };

        assert_eq!(info, foo);
    }
}
