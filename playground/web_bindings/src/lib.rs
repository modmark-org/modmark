use std::cell::RefCell;
use std::path::Path;

use modmark_core::{eval, eval_no_document, Context, CoreError, DenyAllResolver, OutputFormat};
use parser::ParseError;
use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use wasm_bindgen::prelude::*;
use wasmer_vfs::FileSystem;

thread_local! {
    static CONTEXT: RefCell<Context<DenyAllResolver>> = RefCell::new(Context::new_without_resolver().unwrap())
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

#[wasm_bindgen]
pub fn transpile_no_document(source: &str, format: &str) -> Result<String, PlaygroundError> {
    let result = CONTEXT.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        eval_no_document(source, &mut ctx, &OutputFormat::new(format))
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

#[wasm_bindgen]
pub fn get_file_list(path: &str) -> String {
    CONTEXT.with(|ctx| {
        let mut html = String::new();
        let ctx = ctx.borrow();

        for (name, is_folder) in ctx.filesystem.list_dir(path) {
            let icon = if is_folder {
                "<span class=\"material-symbols-outlined\">folder_open</span>"
            } else {
                "<span class=\"material-symbols-outlined\">description</span>"
            };

            let id = if is_folder {
                format!("dir-{name}")
            } else {
                format!("file-{name}")
            };

            let entry_name = if is_folder {
                format!("<div class=\"dir-name\">{name}</div>")
            } else {
                format!("<div class=\"file-name\">{name}</div>")
            };

            let rename_button = format!(
                "<button class=\"rename-button\" name=\"{id}\">\
                <span class=\"material-symbols-outlined\">edit</span>\
                </button>"
            );
            let delete_button = format!(
                "<button class=\"remove-button\" name=\"{id}\">\
                <span class=\"material-symbols-outlined\">delete</span>\
                </button>"
            );
            html = format!(
                "{html}<div class=\"dir-entry\">\
                {icon}\
                {entry_name}\
                {rename_button}\
                {delete_button}\
                </div>");
        }
        json!({"list": html}).to_string()
    })
}

#[wasm_bindgen]
pub fn add_file(path: &str, data: &[u8]) {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        ctx.filesystem.create_file(path, data).unwrap();
    })
}

#[wasm_bindgen]
pub fn add_folder(path: &str) {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        ctx.filesystem.create_dir(Path::new(path)).unwrap();
    })
}

#[wasm_bindgen]
pub fn rename_entry(path: &str, new_path: &str) {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        ctx.filesystem.rename(Path::new(path), Path::new(new_path)).unwrap();
    })
}

#[wasm_bindgen]
pub fn remove_file(path: &str) {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        ctx.filesystem.remove_file(Path::new(path)).unwrap();
    })
}

#[wasm_bindgen]
pub fn remove_dir(path: &str) {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        ctx.filesystem.remove_dir(Path::new(path)).unwrap();
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
