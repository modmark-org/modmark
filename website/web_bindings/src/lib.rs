use modmark_core::{
    eval, eval_no_document, Context, CoreError, DefaultAccessManager, Element, GranularId,
    OutputFormat,
};
use once_cell::sync::Lazy;
use parser::ParseError;
use rand::{Rng, rngs::ThreadRng};
use serde::Serialize;
use serde_json::json;
use std::cell::RefCell;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use thiserror::Error;
use wasm_bindgen::prelude::*;
use wasmer_vfs::FileSystem;
use web_resolver::WebResolver;

mod web_resolver;
thread_local! {
    static CONTEXT: RefCell<Context<WebResolver, DefaultAccessManager>> =
        RefCell::new(Context::new(WebResolver, DefaultAccessManager).unwrap())
}

// AtomicUSize::default isn't const so we can't use Lazy::default
pub static REQUESTS_LEFT: Lazy<AtomicUsize> = Lazy::new(AtomicUsize::default);

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
                let json_errors = errors
                    .into_iter()
                    .map(|error| {
                        json!({
                            "message": error.to_string(),
                            "raw": format!("{error:#?}")
                        })
                    })
                    .collect::<Vec<_>>();

                JsValue::from_str(
                    serde_json::to_string(&json!({"type":"compilationError", "data": json_errors}))
                        .unwrap()
                        .as_str(),
                )
            }
            PlaygroundError::Parsing(error) => {
                let json_error = json!({
                    "message": error.to_string(),
                    "raw": format!("{error:#?}")
                });

                JsValue::from_str(
                    serde_json::to_string(&json!({"type":"parsingError", "data": json_error}))
                        .unwrap()
                        .as_str(),
                )
            }
            PlaygroundError::NoResult => JsValue::from_str(
                serde_json::to_string(&json!({"type":"noResult"}))
                    .unwrap()
                    .as_str(),
            ),
        }
    }
}

pub fn recompile() {
    // We want to send back a message telling the Playground it is ready to recompile
    // I haven't figured out how to get the current ServiceWorker to be able to make a message
    // So the current implementation polls is_ready_for_recompile function
}

#[wasm_bindgen]
pub fn get_req_left() -> usize {
    REQUESTS_LEFT.fetch_add(0, Ordering::Acquire)
}

#[wasm_bindgen]
pub fn is_ready_for_recompile() -> bool {
    REQUESTS_LEFT.fetch_add(0, Ordering::Acquire) == 0
}

#[wasm_bindgen]
pub fn ast(source: &str) -> Result<String, PlaygroundError> {
    let (ast, _) = parser::parse_with_config(source)?;
    Ok(ast.tree_string())
}

#[wasm_bindgen]
pub fn ast_debug(source: &str) -> Result<String, PlaygroundError> {
    let (ast, _) = parser::parse_with_config(source)?;
    Ok(format!("{ast:#?}"))
}

#[wasm_bindgen]
pub fn blank_context() {
    CONTEXT.with(|ctx| {
        ctx.replace_with(|_| Context::new_without_standard(WebResolver, DefaultAccessManager))
    });
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
    // For IDs, we use random IDs in range 5000 to 500 000. This will "simulate" unique IDs that
    // are not ordered and whose only property you can depend on is that it is unique
    let mut rng = ThreadRng::default();
    let mut func = move || rng.gen_range(5_000..500_000);

    let result = CONTEXT.with(|ctx| {
        let ctx = ctx.borrow_mut();
        let doc = Element::try_from_ast(parser::parse_with_config(source)?.0, GranularId::root())
            .map_err(|e| vec![e])?;
        ctx.serialize_element(&doc, &OutputFormat::new("html"), &mut func)
            .map_err(|e| vec![e])
            .map_err(|e| Into::<PlaygroundError>::into(e))
    })?;

    Ok(result)
}

/// Read a file and load the packages found
/// in the config, but never evaluate the actual document
#[wasm_bindgen]
pub fn configure_from_source(source: &str) -> Result<bool, PlaygroundError> {
    CONTEXT
        .with(|ctx| {
            let mut ctx = ctx.borrow_mut();
            let (_, config) = parser::parse_with_config(source).unwrap();
            ctx.configure(config)
        })
        .map_err(Into::into)
}

#[wasm_bindgen]
pub fn package_info() -> String {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        let lock = ctx.package_store.lock().unwrap();
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
pub fn remove_folder(path: &str) -> String {
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
