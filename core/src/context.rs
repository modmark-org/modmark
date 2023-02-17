use crate::Element;
use parser::ModuleArguments;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{Read, Write},
};
#[cfg(feature = "native")]
use wasmer::Cranelift;
use wasmer::{Instance, Store};
use wasmer_wasi::{Pipe, WasiState};

use crate::{ArgInfo, CoreError, OutputFormat, Package, PackageInfo, Transform};

#[derive(Debug)]
pub struct Context {
    packages: HashMap<String, Package>,
    transforms: HashMap<(String, OutputFormat), (Transform, Package)>,
    store: Store,
}

impl Context {
    pub fn new() -> Self {
        Context {
            packages: HashMap::new(),
            transforms: HashMap::new(),
            store: get_new_store(),
        }
    }

    pub fn load_default_packages(&mut self) {
        self.load_package(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/table/wasm32-wasi/release/table.wasm"
        )))
        .expect("Failed to load standard table module");
        self.load_package(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/html/wasm32-wasi/release/html.wasm"
        )))
        .expect("Failed to load standard html module");
    }

    pub fn load_package(&mut self, wasm_source: &[u8]) -> Result<(), CoreError> {
        let pkg = Package::new(wasm_source, &mut self.store)?;

        self.packages
            .insert(pkg.info.as_ref().name.to_string(), pkg.clone());

        // Go through all transforms that the package supports and add them
        // to the Context.
        for transform in &pkg.info.transforms {
            let Transform {
                from,
                to,
                arguments: _,
            } = transform;

            for output_format in to {
                // Ensure that there are no other packages responsible for transforming to the same output format.
                if self
                    .transforms
                    .contains_key(&(from.clone(), output_format.clone()))
                {
                    return Err(CoreError::OccupiedTransform(
                        from.clone(),
                        output_format.to_string(),
                        pkg.info.name.clone(),
                    ));
                }

                self.transforms.insert(
                    (from.to_string(), output_format.clone()),
                    (transform.clone(), pkg.clone()),
                );
            }
        }

        Ok(())
    }

    /// Transform an Element by using the loaded packages. The function will return a
    /// Element::Compound.
    pub fn transform(
        &mut self,
        from: &Element,
        output_format: &OutputFormat,
    ) -> Result<Element, CoreError> {
        use Element::*;

        match from {
            Compound(_) => unreachable!("Should not transform compound element"),
            Node { name, children: _ }
            | ModuleInvocation {
                name,
                args: _,
                body: _,
                one_line: _,
            } => {
                // We find the package responsible for this transform
                let Some((transform, package)) = self.transforms.get(&(name.clone(), output_format.clone())) else {
                    return Err(CoreError::MissingTransform(name.clone(), output_format.to_string()));
                };

                let mut input = Pipe::new();
                let mut output = Pipe::new();
                write!(
                    &mut input,
                    "{}",
                    serialize_element(from, &transform.arguments)?
                )?;

                let wasi_env = WasiState::new("")
                    .stdin(Box::new(input))
                    .stdout(Box::new(output.clone()))
                    .args(["transform", name, &output_format.to_string()])
                    .finalize(&mut self.store)?;

                let import_object =
                    wasi_env.import_object(&mut self.store, &package.wasm_module)?;
                let instance =
                    Instance::new(&mut self.store, &package.wasm_module, &import_object)?;

                // Attach the memory export
                let memory = instance.exports.get_memory("memory")?;
                wasi_env
                    .data_mut(&mut self.store)
                    .set_memory(memory.clone());

                // Call the main entry point of the program
                let main_fn = instance.exports.get_function("_start")?;
                main_fn.call(&mut self.store, &[])?;

                // Read the output of the package from stdout
                let result = {
                    let mut buffer = String::new();
                    output.read_to_string(&mut buffer)?;
                    deserialize_compound(&buffer)
                };

                result
            }
        }
    }

    /// Borrow information about a package with a given name
    pub fn get_package_info(&self, name: &str) -> Option<&PackageInfo> {
        self.packages.get(name).map(|pkg| pkg.info.as_ref())
    }
}

impl Default for Context {
    /// A Context with all default packages lodaded
    fn default() -> Self {
        let mut ctx = Self::new();
        ctx.load_default_packages();
        ctx
    }
}

/// Get a new using different compilers depending
/// if we are using the "web" or "native" feature
fn get_new_store() -> Store {
    #[cfg(feature = "web")]
    return Store::new();

    #[cfg(feature = "native")]
    return Store::new(Cranelift::new());
}

/// This enum is in the same shape as the json objects that
/// will be sent and recieved when communicating with packages
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum JsonEntry {
    ParentNode {
        name: String,
        children: Vec<Self>,
    },
    Module {
        name: String,
        #[serde(default)]
        data: String,
        #[serde(default)]
        arguments: HashMap<String, String>,
        #[serde(default = "default_inline")]
        inline: bool,
    },
}

/// This is just a helper to ensure that omited "inline" fields
/// default to true.
fn default_inline() -> bool {
    true
}

/// Serialize and element into a string that can be sent to a package
fn serialize_element(element: &Element, args_info: &Vec<ArgInfo>) -> Result<String, CoreError> {
    let entry = element_to_entry(element, args_info)?;
    serde_json::to_string(&entry).map_err(|e| e.into())
}

/// Deserialize a compound (i.e a list of JsonEntries) that are recived from a package
fn deserialize_compound(input: &str) -> Result<Element, CoreError> {
    let entries: Vec<JsonEntry> = serde_json::from_str(input)?;

    // Convert the parsed entries into real Elements
    let elements: Vec<Element> = entries.into_iter().map(entry_to_element).collect();
    Ok(Element::Compound(elements))
}

/// Convert an JsonEntry to a Element
fn entry_to_element(entry: JsonEntry) -> Element {
    match entry {
        JsonEntry::ParentNode { name, children } => Element::Node {
            name,
            children: children
                .into_iter()
                .map(|child| entry_to_element(child))
                .collect(),
        },
        JsonEntry::Module {
            name,
            data,
            arguments,
            inline,
        } => Element::ModuleInvocation {
            name,
            args: ModuleArguments {
                positioned: None,
                named: Some(arguments),
            },
            body: data,
            one_line: inline,
        },
    }
}

/// Convert a Element into a JsonEntry.
/// The args_info is needed to convert positional arguments into key-value pairs.
fn element_to_entry(element: &Element, args_info: &Vec<ArgInfo>) -> Result<JsonEntry, CoreError> {
    match element {
        // When the eval function naivly evaluates all children before a parent compund
        // nodes should never be present here. This may however change in the future.
        Element::Compound(_) => unreachable!(),
        Element::Node { name, children } => {
            let converted_children: Result<Vec<JsonEntry>, CoreError> = children
                .into_iter()
                .map(|child| element_to_entry(child, args_info))
                .collect();

            Ok(JsonEntry::ParentNode {
                name: name.clone(),
                children: converted_children?,
            })
        }
        Element::ModuleInvocation {
            name: module_name,
            args,
            body,
            one_line,
        } => {
            let mut arguments: HashMap<String, String> = HashMap::new();

            // Look up the name of all the positional arguments specified and insert
            // them into the arguments map
            if let Some(positioned) = &args.positioned {
                for (value, arg_info) in positioned.into_iter().zip(args_info) {
                    let ArgInfo {
                        name,
                        default: _,
                        description: _,
                    } = arg_info;
                    arguments.insert(name.clone(), value.clone());
                }
            }

            // Also, add the rest of the key=value paired arguments
            if let Some(named) = &args.named {
                for (name, value) in named {
                    if arguments.contains_key(name) {
                        return Err(CoreError::RepeatedArgument(name.clone(), name.clone()));
                    }
                    arguments.insert(name.clone(), value.clone());
                }
            }

            Ok(JsonEntry::Module {
                name: module_name.clone(),
                arguments,
                data: body.clone(),
                inline: *one_line,
            })
        }
    }
}
