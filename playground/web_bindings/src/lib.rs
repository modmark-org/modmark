use core::{eval, Context, CoreError, OutputFormat};
use parser::ParseError;
use std::cell::RefCell;
use thiserror::Error;
use wasm_bindgen::prelude::*;

thread_local! {
    static CONTEXT: RefCell<Context> = RefCell::new(Context::default());
}

#[derive(Error, Debug)]
pub enum PlaygroundError {
    #[error("Failed to evaluate the document")]
    Core(#[from] CoreError),
    #[error("Failed to parse")]
    Parsing(#[from] ParseError),
}

impl From<PlaygroundError> for JsValue {
    fn from(error: PlaygroundError) -> Self {
        match error {
            PlaygroundError::Core(error) => {
                JsValue::from_str(&format!("<p>{error}</p><pre>{error:#?}</pre>"))
            }
            PlaygroundError::Parsing(error) => {
                JsValue::from_str(&format!("<p>{error}</p><pre>{error:#?}</pre>"))
            }
        }
    }
}

#[wasm_bindgen]
pub fn ast(source: &str) -> Result<String, PlaygroundError> {
    set_panic_hook();
    let document = parser::parse_to_ast(source)?;
    Ok(document.tree_string())
}

#[wasm_bindgen]
pub fn ast_debug(source: &str) -> Result<String, PlaygroundError> {
    let document = parser::parse_to_ast(source)?;
    Ok(format!("{document:#?}"))
}

#[wasm_bindgen]
pub fn transpile(source: &str) -> Result<String, PlaygroundError> {
    let result = CONTEXT.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        eval(source, &mut ctx, &OutputFormat::new("html"))
    });

    Ok(result?)
}

#[wasm_bindgen]
pub fn inspect_context() -> String {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow_mut();
        format!("{ctx:#?}")
    })
}

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    console_error_panic_hook::set_once();
}
