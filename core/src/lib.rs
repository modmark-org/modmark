use parser::Element;

mod context;
mod error;
mod loaded_module;

pub use context::Context;
pub use error::CoreError;
pub use loaded_module::{Arg, LoadedModule, ModuleInfo, NodeName, Transform};

/// Evaluates a document using the given context
pub fn eval(_document: &Element, _ctx: &mut Context) -> String {
    "TODO".to_string()
}
