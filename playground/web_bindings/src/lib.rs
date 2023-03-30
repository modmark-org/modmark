use std::cell::RefCell;
use std::path::Path;

use modmark_core::{
    eval, eval_no_document, Context, CoreError, DefaultAccessManager, OutputFormat,
};
use parser::ParseError;
use serde::Serialize;
use thiserror::Error;
use wasm_bindgen::prelude::*;

use wasmer_vfs::FileSystem;

mod web_resolve;

thread_local! {
    static CONTEXT: RefCell<Context<web_resolve::WebResolve, DefaultAccessManager>> =
        RefCell::new(Context::new(web_resolve::WebResolve, DefaultAccessManager).unwrap())
}

#[derive(Error, Debug)]
pub enum PlaygroundError {
    #[error("Failed to evaluate the document")]
    Core(Vec<CoreError>),
    #[error("Failed to parse")]
    Parsing(#[from] ParseError),
    #[error("No result")]
    NoResult,
}

impl From<Vec<CoreError>> for PlaygroundError {
    fn from(value: Vec<CoreError>) -> Self {
        PlaygroundError::Core(value)
    }
}

impl From<PlaygroundError> for JsValue {
    fn from(error: PlaygroundError) -> Self {
        match error {
            PlaygroundError::Core(errors) => {
                let mut str = String::new();
                for error in errors {
                    str.push_str(&format!("<p>{error}</p><pre>{error:#?}</pre>"));
                }
                JsValue::from_str(&str)
            }
            PlaygroundError::Parsing(error) => {
                JsValue::from_str(&format!("<p>{error}</p><pre>{error:#?}</pre>"))
            }
            PlaygroundError::NoResult => {
                JsValue::from_str(&format!("<p>{error}</p><pre>No result</pre>"))
            }
        }
    }
}

#[wasm_bindgen]
pub fn ast(source: &str) -> Result<String, PlaygroundError> {
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
    let result = CONTEXT
        .with(|ctx| {
            let mut ctx = ctx.borrow_mut();
            eval(source, &mut ctx, &OutputFormat::new(format))
        })?
        .ok_or(PlaygroundError::NoResult)?;

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

#[wasm_bindgen]
pub fn transpile_no_document(source: &str, format: &str) -> Result<String, PlaygroundError> {
    set_panic_hook();
    let result = CONTEXT
        .with(|ctx| {
            let mut ctx = ctx.borrow_mut();
            eval_no_document(source, &mut ctx, &OutputFormat::new(format))
        })?
        .ok_or(PlaygroundError::NoResult)?;

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
        let doc = parser::parse(source)?.try_into().map_err(|e| vec![e])?;
        ctx.serialize_element(&doc, &OutputFormat::new("html"))
            .map_err(|e| vec![e])
            .map_err(|e| Into::<PlaygroundError>::into(e))
    })?;

    Ok(result)
}

#[wasm_bindgen]
pub fn package_info() -> String {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        let lock = ctx.package_manager.lock().unwrap();
        serde_json::to_string(&lock.get_all_package_info()).unwrap()
    })
}

#[wasm_bindgen]
pub fn get_file_list(path: &str) -> String {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        // Placeholder error handling, revisit if it becomes important
        match ctx.filesystem.list_dir(Path::new(path)) {
            Ok(entries) => serde_json::to_string(&entries).unwrap(),
            Err(_) => String::new(),
        }
    })
}

#[wasm_bindgen]
pub fn add_file(path: &str, data: &[u8]) -> String {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        match ctx.filesystem.create_file(Path::new(path), data) {
            Ok(_) => String::new(),
            Err(e) => e.to_string(),
        }
    })
}

#[wasm_bindgen]
pub fn add_folder(path: &str) -> String {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        match ctx.filesystem.create_dir(Path::new(path)) {
            Ok(_) => String::new(),
            Err(e) => e.to_string(),
        }
    })
}

#[wasm_bindgen]
pub fn rename_entry(path: &str, new_path: &str) -> String {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        match ctx.filesystem.rename(Path::new(path), Path::new(new_path)) {
            Ok(_) => String::new(),
            Err(e) => e.to_string(),
        }
    })
}

#[wasm_bindgen]
pub fn remove_file(path: &str) -> String {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        match ctx.filesystem.remove_file(Path::new(path)) {
            Ok(_) => String::new(),
            Err(e) => e.to_string(),
        }
    })
}

#[wasm_bindgen]
pub fn remove_dir(path: &str) -> String {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        match ctx.filesystem.remove_dir(Path::new(path)) {
            Ok(_) => String::new(),
            Err(e) => e.to_string(),
        }
    })
}

#[wasm_bindgen]
pub fn read_file(path: &str) -> Vec<u8> {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        // Placeholder error handling, revisit if it becomes important
        match ctx.filesystem.read_file(Path::new(path)) {
            Ok(data) => data,
            Err(_) => vec![],
        }
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
