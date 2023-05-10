use std::collections::HashMap;

use serde_json::Value;
#[cfg(feature = "native")]
use wasmer::Engine;

use parser::ModuleArguments;

use crate::context::Issue;
use crate::element::GranularId;
use crate::package::{ArgValue, PrimitiveArgType, TransformType};
use crate::package_store::PackageStore;
use crate::std_packages_macros::{define_native_packages, define_standard_package_loader};
use crate::variables::{self, ConstantAccess, ListAccess, SetAccess, VarAccess, VarType};
use crate::{ArgInfo, Context, CoreError, Element, OutputFormat, PackageInfo, Transform};

// Here, all standard packages are declared. The macro expands to one function
// which takes a &mut Context and loads it with the given standard package. It is important that the
// package with a given name both is in a folder with that name, containing a cargo package with
// that name. Otherwise, the module won't be found
define_standard_package_loader! {
    "table",
    "html",
    "latex",
    "link",
    "list",
    "code",
    "math",
    "layout",
    "files",
    "flow",
    "bibliography",
    "plot",
}

// Here, all native packages are declared. The macro expands to two functions,
// one being a function returning the manifests for those packages, and the other
// being the entry point to run these packages.
define_native_packages! {
    "core" => {
        desc: "Provides core functionality such as raw output, errors and warnings",
        transforms: [
            {
                name: "raw",
                desc: "Outputs the body text as-is into the output document",
                unknown_content: false,
                vars: [],
                args: vec![],
                func: native_raw
            },
            {
                name: "warning",
                desc: "Adds a compilation warning to the list of all warnings that have occurred during compilation",
                unknown_content: false,
                vars: [],
                args: vec![
                    ArgInfo {
                        name: "source".to_string(),
                        default: Some(Value::String("<unknown>".to_string())),
                        description: "The source module/parent responsible for the warning".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                    ArgInfo {
                        name: "target".to_string(),
                        default: Some(Value::String("<unknown>".to_string())),
                        description: "The target output format when the warning was generated".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                    ArgInfo {
                        name: "input".to_string(),
                        default: Some(Value::String("<unknown>".to_string())),
                        description: "The input given to the module when it failed".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                ],
                func: native_warn
            },
            {
                name: "error",
                desc: "Adds a compilation error to the list of all errors that have occurred during compilation",
                unknown_content: false,
                vars: [],
                args: vec![
                    ArgInfo {
                        name: "source".to_string(),
                        default: Some(Value::String("<unknown>".to_string())),
                        description: "The source module/parent responsible for the error".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                    ArgInfo {
                        name: "target".to_string(),
                        default: Some(Value::String("<unknown>".to_string())),
                        description: "The target output format when the error was generated".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                    ArgInfo {
                        name: "input".to_string(),
                        default: Some(Value::String("<unknown>".to_string())),
                        description: "The input given to the module when it failed".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                ],
                func: native_err
            }
        ]
    }
    "reparse" => {
        desc: "Provides an interface to the built-in ModMark parser",
        transforms: [
            {
                name: "inline_content",
                desc: "Parses the content as inline-content, as if it was in a paragraph. The result may contain text, smart punctuation, inline module expressions and tags",
                unknown_content: true,
                vars: [],
                args: vec![],
                func: native_inline_content
            },
            {
                name: "block_content",
                desc: "Parses the content as block-content, as if it was in the body of the document. The result may contain paragraphs containing inline content and multiline module expressions",
                unknown_content: true,
                vars: [],
                args: vec![],
                func: native_block_content
            }
        ]
    }
    "variables" => {
        desc: "Read and write to environment variables.",
        transforms: [
            {
                name: "const-decl",
                desc: "Declare a constant",
                unknown_content: false,
                vars: [("$name".to_string(), VarAccess::Constant(ConstantAccess::Declare))],
                args: vec![
                    ArgInfo {
                        name: "name".to_string(),
                        default: None,
                        description: "The name to declare".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    }
                ],
                func: const_decl
            },
            {
                name: "const-read",
                desc: "Read a constant",
                unknown_content: false,
                vars: [("$name".to_string(), VarAccess::Constant(ConstantAccess::Read))],
                args: vec![
                    ArgInfo {
                        name: "name".to_string(),
                        default: None,
                        description: "The name of the constant to read".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                    ArgInfo {
                        name: "default".to_string(),
                        default: Some(Value::from("<log error>")),
                        description: "A fallback text to used if the variable is undefined. If a 'default' is not provided a error will be created.".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    }
                ],
                func: const_read
            },
            {
                name: "list-push",
                desc: "Push a string to a list",
                unknown_content: false,
                vars: [("$name".to_string(), VarAccess::List(ListAccess::Push))],
                args: vec![
                    ArgInfo {
                        name: "name".to_string(),
                        default: None,
                        description: "The name of the list".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    }
                ],
                func: list_push
            },
            {
                name: "list-read",
                desc: "Read a list. The list will be serialized as a JSON array.",
                unknown_content: false,
                vars: [("$name".to_string(), VarAccess::List(ListAccess::Read))],
                args: vec![
                    ArgInfo {
                        name: "name".to_string(),
                        default: None,
                        description: "The name of the list to read".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                    ArgInfo {
                        name: "default".to_string(),
                        default: Some(Value::from("<log error>")),
                        description: "A fallback text to used if the variable is undefined. If a 'default' is not provided a error will be created.".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    }
                ],
                func: list_read
            },
            {
                name: "set-add",
                desc: "Add a string to a set. Note that sets do not contains duplicates and are not ordered.",
                unknown_content: false,
                vars: [("$name".to_string(), VarAccess::Set(SetAccess::Add))],
                args: vec![
                    ArgInfo {
                        name: "name".to_string(),
                        default: None,
                        description: "The name of the set".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    }
                ],
                func: set_add
            },
            {
                name: "set-read",
                desc: "Read a set. The set will be serialized as a JSON array. Note that sets do not follow a deterministic order.",
                unknown_content: false,
                vars: [("$name".to_string(), VarAccess::Set(SetAccess::Read))],
                args: vec![
                    ArgInfo {
                        name: "name".to_string(),
                        default: None,
                        description: "The name of the set to read".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    },
                    ArgInfo {
                        name: "default".to_string(),
                        default: Some(Value::from("<log error>")),
                        description: "A fallback text to used if the variable is undefined. If a 'default' is not provided a error will be created.".to_string(),
                        r#type: PrimitiveArgType::String.into()
                    }
                ],
                func: set_read
            },
        ]
    }
}

/// Returns a string containing the body of this invocation. This is the "leaf" call; no tree will
/// be created as a result of this call
pub fn native_raw<T, U>(
    _ctx: &mut Context<T, U>,
    body: &str,
    _args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
    _id: &GranularId,
) -> Result<Element, CoreError> {
    Ok(Element::Raw(body.to_owned()))
}

/// Re-parses the content as block content (a paragraph with tags, modules etc) and
/// returns the resulting compound element containing the contents of the paragraph
pub fn native_inline_content<T, U>(
    _ctx: &mut Context<T, U>,
    body: &str,
    _args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
    id: &GranularId,
) -> Result<Element, CoreError> {
    let elements = parser::parse_inline(body)?
        .into_iter()
        .zip(id.children())
        .map(|(ast, id)| Element::try_from_ast(ast, id))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Element::Compound(elements))
}

/// Re-parses the content as block content (multiple paragraph or multiline module invocations) and
/// returns the resulting compound element
pub fn native_block_content<T, U>(
    _ctx: &mut Context<T, U>,
    body: &str,
    _args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
    id: &GranularId,
) -> Result<Element, CoreError> {
    let elements = parser::parse_blocks(body)?
        .into_iter()
        .zip(id.children())
        .map(|(ast, id)| Element::try_from_ast(ast, id))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Element::Compound(elements))
}

/// Helper function to create text elements
fn text_element(contents: String, id: GranularId) -> Element {
    Element::Module {
        name: "__text".to_string(),
        args: Default::default(),
        body: contents,
        inline: true,
        id,
    }
}

/// Declare a constant
pub fn const_decl<T, U>(
    ctx: &mut Context<T, U>,
    value: &str,
    args: HashMap<String, ArgValue>,
    _: bool,
    _: &OutputFormat,
    _: &GranularId,
) -> Result<Element, CoreError> {
    let name = args.get("name").unwrap().as_str().unwrap();
    ctx.state.variables.constant_declare(name, value)?;

    Ok(Element::Compound(vec![]))
}

/// Read a constant
pub fn const_read<T, U>(
    ctx: &mut Context<T, U>,
    _: &str,
    args: HashMap<String, ArgValue>,
    _: bool,
    format: &OutputFormat,
    id: &GranularId,
) -> Result<Element, CoreError> {
    let name = args.get("name").unwrap().as_str().unwrap();

    match ctx.state.variables.get(name) {
        Some(value @ variables::Value::Constant(_)) => {
            Ok(text_element(value.to_string(), id.clone()))
        }
        Some(value) => Err(CoreError::TypeMismatch {
            name: name.to_string(),
            expected_type: VarType::Constant,
            present_type: value.get_type(),
        }),
        None => {
            let default = args.get("default").and_then(ArgValue::as_str).unwrap();
            if default == "<log error>" {
                // No default was provided as fallback, let's log a error
                Ok(get_read_error(name, "const-read", id, format))
            } else {
                Ok(text_element(default.to_string(), id.clone()))
            }
        }
    }
}

/// Push a value to a list
pub fn list_push<T, U>(
    ctx: &mut Context<T, U>,
    value: &str,
    args: HashMap<String, ArgValue>,
    _: bool,
    _: &OutputFormat,
    _: &GranularId,
) -> Result<Element, CoreError> {
    let name = args.get("name").unwrap().as_str().unwrap();
    ctx.state.variables.list_push(name, value)?;
    Ok(Element::Compound(vec![]))
}

/// Read a list of values
pub fn list_read<T, U>(
    ctx: &mut Context<T, U>,
    _: &str,
    args: HashMap<String, ArgValue>,
    _: bool,
    format: &OutputFormat,
    id: &GranularId,
) -> Result<Element, CoreError> {
    let name = args.get("name").unwrap().as_str().unwrap();

    match ctx.state.variables.get(name) {
        Some(value @ variables::Value::List(_)) => Ok(text_element(value.to_string(), id.clone())),
        Some(value) => Err(CoreError::TypeMismatch {
            name: name.to_string(),
            expected_type: VarType::List,
            present_type: value.get_type(),
        }),
        None => {
            let default = args.get("default").and_then(ArgValue::as_str).unwrap();
            if default == "<log error>" {
                // No default was provided as fallback, let's log a error
                Ok(get_read_error(name, "list-read", id, format))
            } else {
                Ok(text_element(default.to_string(), id.clone()))
            }
        }
    }
}

/// Add a string to a set
pub fn set_add<T, U>(
    ctx: &mut Context<T, U>,
    value: &str,
    args: HashMap<String, ArgValue>,
    _: bool,
    _: &OutputFormat,
    _: &GranularId,
) -> Result<Element, CoreError> {
    let name = args.get("name").unwrap().as_str().unwrap();
    ctx.state.variables.set_add(name, value)?;
    Ok(Element::Compound(vec![]))
}

/// Read a list of values
pub fn set_read<T, U>(
    ctx: &mut Context<T, U>,
    _: &str,
    args: HashMap<String, ArgValue>,
    _: bool,
    format: &OutputFormat,
    id: &GranularId,
) -> Result<Element, CoreError> {
    let name = args.get("name").unwrap().as_str().unwrap();

    match ctx.state.variables.get(name) {
        Some(value @ variables::Value::Set(_)) => Ok(text_element(value.to_string(), id.clone())),
        Some(value) => Err(CoreError::TypeMismatch {
            name: name.to_string(),
            expected_type: VarType::Set,
            present_type: value.get_type(),
        }),
        None => {
            let default = args.get("default").and_then(ArgValue::as_str).unwrap();
            if default == "<log error>" {
                // No default was provided as fallback, let's log a error
                Ok(get_read_error(name, "set-read", id, format))
            } else {
                Ok(text_element(default.to_string(), id.clone()))
            }
        }
    }
}

fn get_read_error(
    variable_name: &str,
    module_name: &str,
    id: &GranularId,
    format: &OutputFormat,
) -> Element {
    Element::Module {
        name: "error".to_string(),
        args: ModuleArguments {
            positioned: None,
            named: Some({
                let mut map = HashMap::new();
                map.insert("source".to_string(), module_name.to_string());
                map.insert("target".to_string(), format.to_string());
                // read modules do not take any input
                map.insert("input".to_string(), "".to_string());
                map
            }),
        },
        body: format!("Attempted to read undefined variable '{variable_name}'. Try defining the variable or provide a 'default' argument to the read module."),
        inline: true,
        id: id.clone(),
    }
}

pub fn native_warn<T, U>(
    ctx: &mut Context<T, U>,
    body: &str,
    mut args: HashMap<String, ArgValue>,
    _inline: bool,
    _output_format: &OutputFormat,
    _id: &GranularId,
) -> Result<Element, CoreError> {
    // Push the issue to warnings
    ctx.state.warnings.push(Issue {
        source: args.remove("source").unwrap().get_string().unwrap(),
        target: args.remove("target").unwrap().get_string().unwrap(),
        description: body.to_string(),
        input: args
            .remove("input")
            .map(|v| v.get_string().unwrap())
            .and_then(|s| (s != "<unknown>").then_some(s)),
    });

    // Return no new nodes
    Ok(Element::Compound(vec![]))
}

pub fn native_err<T, U>(
    ctx: &mut Context<T, U>,
    body: &str,
    args: HashMap<String, ArgValue>,
    inline: bool,
    output_format: &OutputFormat,
    id: &GranularId,
) -> Result<Element, CoreError> {
    let source = args.get("source").unwrap().clone().get_string().unwrap();
    let target = args.get("target").unwrap().clone().get_string().unwrap();
    let input = args
        .get("input")
        .map(|v| v.clone().get_string().unwrap())
        .and_then(|s| (s != "<unknown>").then_some(s));

    // Push the issue to errors
    ctx.state.errors.push(Issue {
        source: source.to_string(),
        target,
        description: body.to_string(),
        input,
    });

    // Check if we have an __error transform
    if ctx
        .package_store
        .lock()
        .unwrap()
        .transforms
        .get("__error")
        .and_then(|t| t.find_transform_to(output_format))
        .map_or(true, |t| {
            !matches!(t.0.r#type, TransformType::Module | TransformType::Any)
        })
        || source == "__error"
    {
        // If we don't have, don't add an __error parent since that would yield an CoreError
        // Also if the error did originate from an error module itself, don't generate more and
        // crash
        Ok(Element::Compound(vec![]))
    } else {
        let type_erase = |mut map: HashMap<String, ArgValue>| {
            map.drain()
                .map(|(k, v)| (k, v.into()))
                .collect::<HashMap<String, String>>()
        };

        let args = type_erase(args);

        // If we do, add an __error parent since our output format should
        Ok(Element::Module {
            name: "__error".to_string(),
            body: body.to_string(),
            args: ModuleArguments {
                positioned: None,
                named: Some(args),
            },
            inline,
            id: id.clone(),
        })
    }
}
