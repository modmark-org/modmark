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

    pub fn get_transform_info(
        &self,
        element_name: &str,
        output_format: &OutputFormat,
    ) -> Option<&Transform> {
        // FIXME: would be a lot cleaner if we could do this without cloning the strings
        self.transforms
            .get(&(element_name.to_string(), output_format.clone()))
            .map(|(transform, _)| transform)
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
            Parent {
                name,
                args: _,
                children: _,
            }
            | Module {
                name,
                args: _,
                body: _,
                inline: _,
            } => {
                // We find the package responsible for this transform
                let Some((_, package)) = self.transforms.get(&(name.clone(), output_format.clone())) else {
                    return Err(CoreError::MissingTransform(name.clone(), output_format.to_string()));
                };

                let mut input = Pipe::new();
                let mut output = Pipe::new();
                write!(
                    &mut input,
                    "{}",
                    self.serialize_element(from, output_format)?
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
                    Self::deserialize_compound(&buffer)
                };

                result
            }
        }
    }

    /// Borrow information about a package with a given name
    pub fn get_package_info(&self, name: &str) -> Option<&PackageInfo> {
        self.packages.get(name).map(|pkg| pkg.info.as_ref())
    }

    /// Serialize and element into a string that can be sent to a package
    pub fn serialize_element(
        &self,
        element: &Element,
        output_format: &OutputFormat,
    ) -> Result<String, CoreError> {
        let entry = self.element_to_entry(element, output_format)?;
        serde_json::to_string(&entry).map_err(|e| e.into())
    }

    /// Deserialize a compound (i.e a list of JsonEntries) that are recived from a package
    pub fn deserialize_compound(input: &str) -> Result<Element, CoreError> {
        let entries: Vec<JsonEntry> = serde_json::from_str(input)?;

        // Convert the parsed entries into real Elements
        let elements: Vec<Element> = entries.into_iter().map(Self::entry_to_element).collect();
        Ok(Element::Compound(elements))
    }

    /// Convert an JsonEntry to a Element
    fn entry_to_element(entry: JsonEntry) -> Element {
        match entry {
            JsonEntry::ParentNode {
                name,
                arguments,
                children,
            } => Element::Parent {
                name,
                args: arguments,
                children: children.into_iter().map(Self::entry_to_element).collect(),
            },
            JsonEntry::Module {
                name,
                data,
                arguments,
                inline,
            } => Element::Module {
                name,
                args: ModuleArguments {
                    positioned: None,
                    named: Some(arguments),
                },
                body: data,
                inline,
            },
        }
    }

    /// Convert a Element into a JsonEntry.
    fn element_to_entry(
        &self,
        element: &Element,
        output_format: &OutputFormat,
    ) -> Result<JsonEntry, CoreError> {
        match element {
            // When the eval function naivly evaluates all children before a parent compund
            // nodes should never be present here. This may however change in the future.
            Element::Compound(_) => unreachable!(),
            Element::Parent {
                name: parent_name,
                args,
                children,
            } => {
                let converted_children: Result<Vec<JsonEntry>, CoreError> = children
                    .into_iter()
                    .map(|child| self.element_to_entry(child, output_format))
                    .collect();

                // Collect the arguments and add default values for unspecifed arguments
                let mut collected_args = HashMap::new();
                let mut given_args = args.clone();

                // Get info about what args this parent node
                let empty_vec = vec![];
                let args_info: &Vec<ArgInfo> = self
                    .get_transform_info(parent_name, output_format)
                    .map(|info| info.arguments.as_ref())
                    .unwrap_or(&empty_vec);

                for arg_info in args_info {
                    let ArgInfo {
                        name,
                        default,
                        description: _,
                    } = arg_info;

                    if let Some(value) = given_args.remove(name) {
                        collected_args.insert(name.clone(), value);
                        continue;
                    }

                    if let Some(value) = default {
                        collected_args.insert(name.clone(), value.clone());
                    }

                    return Err(CoreError::MissingArgument(
                        name.clone(),
                        parent_name.clone(),
                    ));
                }

                // Check if there are any stray arguments left that should not be there
                if let Some((key, _)) = given_args.into_iter().next() {
                    return Err(CoreError::InvalidArgument(key, parent_name.clone()));
                }

                Ok(JsonEntry::ParentNode {
                    name: parent_name.clone(),
                    arguments: args.clone(),
                    children: converted_children?,
                })
            }
            Element::Module {
                name: module_name,
                args,
                body,
                inline: one_line,
            } => {
                let empty_vec = vec![];
                let mut pos_args = args.positioned.as_ref().unwrap_or(&empty_vec).iter();
                let mut named_args = args.named.clone().unwrap_or_default();
                let mut collected_args = HashMap::new();

                // Get info about what args this parent node supports
                let empty_vec = vec![];
                let args_info = self
                    .get_transform_info(module_name, output_format)
                    .map(|info| info.arguments.as_ref())
                    .unwrap_or(&empty_vec);

                for arg_info in args_info {
                    let ArgInfo {
                        name,
                        default,
                        description: _,
                    } = arg_info;

                    // First empty the positional arguments
                    if let Some(value) = pos_args.next() {
                        // Check that this key is not repeated later too
                        if named_args.contains_key(name) {
                            return Err(CoreError::RepeatedArgument(
                                name.to_string(),
                                module_name.to_string(),
                            ));
                        }
                        collected_args.insert(name.to_string(), value.clone());
                        continue;
                    }

                    // Check if it was specified as a named key=value pair
                    if let Some(value) = named_args.remove(name) {
                        collected_args.insert(name.to_string(), value.clone());
                        continue;
                    }

                    // Use the default value as a fallback
                    if let Some(value) = default {
                        collected_args.insert(name.to_string(), value.clone());
                        continue;
                    }

                    // Oops, the user seem to be missing a required argument
                    return Err(CoreError::MissingArgument(
                        name.to_string(),
                        module_name.to_string(),
                    ));
                }

                if let Some(value) = pos_args.next() {
                    return Err(CoreError::InvalidArgument(
                        value.to_string(),
                        module_name.to_string(),
                    ));
                }

                if let Some((key, _)) = named_args.iter().next() {
                    return Err(CoreError::InvalidArgument(
                        key.clone(),
                        module_name.to_string(),
                    ));
                }

                Ok(JsonEntry::Module {
                    name: module_name.clone(),
                    arguments: collected_args,
                    data: body.clone(),
                    inline: *one_line,
                })
            }
        }
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
        arguments: HashMap<String, String>,
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
