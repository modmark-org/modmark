use core::{eval, Context};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    static CONTEXT: RefCell<Context> = RefCell::new(Context::default());
}

#[wasm_bindgen]
pub fn parse(source: &str) -> String {
    set_panic_hook();
    let document = parser::parse(source);
    document.tree_string(true)
}

#[wasm_bindgen]
pub fn ast(source: &str) -> String {
    set_panic_hook();
    let document = parser::parse_to_ast(source);
    document.tree_string()
}

#[wasm_bindgen]
pub fn raw_tree(source: &str) -> String {
    set_panic_hook();
    let document = parser::parse(source);
    format!("{document:#?}")
}

#[wasm_bindgen]
pub fn transpile(source: &str) -> String {
    let document = parser::parse(source);
    let mut result = String::new();

    CONTEXT.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        result.push_str(&eval(&document, &mut ctx));
    });

    result
}

#[wasm_bindgen]
pub fn inspect_context() -> String {
    let mut result = String::new();

    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow_mut();
        result.push_str(&format!("{ctx:#?}"));
    });

    result
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
