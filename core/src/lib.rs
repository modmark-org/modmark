use parser::Element;

mod context;
mod error;
mod package;

pub use context::Context;
pub use error::CoreError;
pub use package::{ArgInfo, NodeName, Package, PackageInfo, Transform};

#[cfg(all(feature = "web", feature = "native"))]
compile_error!("feature \"native\" and feature \"web\" cannot be enabled at the same time");



/// Evaluates a document using the given context
pub fn eval(source: &str, ctx: &mut Context) -> String {
    let document = parser::parse(source);
    eval_elem(&document, ctx)
}

pub fn eval_elem(element: &Element, ctx: &mut Context) -> String {
    use Element::*;
    match root {
        Data(_) => {
        },
        Node { name, environment, children } => {
            if evaluated_children.iter().all(|child| false /*Kolla om alla är module 'output' */) {
                // concatenera barnen och ge tillbaka en module output...
            } else {
                unreachable!()
            }
        }
        ModuleInvocation { name, args, body, one_line } => {
            // base case: om det är output
            // då vill vi ge en sträng
            // annars får man kicka igång wasm-runtimen och expandera macrot
            // ..., det vi kallat transform
        },
    };
    todo!()
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
                    to: vec!["table".to_string()],
                    args_info: vec![ArgInfo {
                        name: "border".to_string(),
                        default: Some("black".to_string()),
                        description: "What color the border should be".to_string(),
                    }],
                },
                Transform {
                    from: "table".to_string(),
                    to: vec!["html".to_string(), "latex".to_string()],
                    args_info: vec![],
                },
                Transform {
                    from: "row".to_string(),
                    to: vec!["html".to_string(), "latex".to_string()],
                    args_info: vec![],
                },
            ],
        };

        assert_eq!(info, foo);
    }
}
