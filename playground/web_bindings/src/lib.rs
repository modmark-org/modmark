use core::{eval, Context, CoreError, OutputFormat};
use std::cell::RefCell;

use parser::ParseError;
use serde::Serialize;
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
    let document = parser::parse(source)?;
    Ok(document.tree_string())
}

#[wasm_bindgen]
pub fn ast_debug(source: &str) -> Result<String, PlaygroundError> {
    let document = parser::parse(source)?;
    Ok(format!("{document:#?}"))
}

#[derive(Serialize)]
struct Transpile {
    content: String,
    warnings: Vec<String>,
    errors: Vec<String>,
}

#[wasm_bindgen]
pub fn transpile(source: &str, format: &str) -> Result<String, PlaygroundError> {
    let result = CONTEXT.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        eval(source, &mut ctx, &OutputFormat::new(format))
    })?;

    let warnings = result
        .1
        .warnings
        .iter()
        .map(|issue| escape(issue.to_string()))
        .collect();
    let errors = result
        .1
        .errors
        .iter()
        .map(|issue| escape(issue.to_string()))
        .collect();
    let transpile = Transpile {
        content: result.0,
        warnings,
        errors,
    };
    Ok(serde_json::to_string(&transpile).unwrap())
}

fn escape(text: String) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace("\r\n", "\\n")
        .replace('\n', "\\n")
}

#[wasm_bindgen]
pub fn json_output(source: &str) -> Result<String, PlaygroundError> {
    let result = CONTEXT.with(|ctx| {
        let ctx = ctx.borrow_mut();
        let doc = parser::parse(source)?.try_into()?;
        ctx.serialize_element(&doc, &OutputFormat::new("html"))
    })?;

    Ok(result)
}

#[wasm_bindgen]
pub fn package_info() -> String {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        serde_json::to_string(&ctx.get_all_package_info()).unwrap()
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
