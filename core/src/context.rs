use std::{
    collections::HashMap,
    io::{Read, Write},
};

use parser::{Element, ModuleArguments};
use serde::Deserialize;
use serde_json::json;
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

    /// Transform a node into another node by using the available transforms
    /// FIXME: maybe should own the Element.
    pub fn transform(
        &mut self,
        from: &Element,
        output_format: &OutputFormat,
    ) -> Result<Element, CoreError> {
        use Element::*;

        match from {
            Data(_) => unreachable!("No transform on data node"),
            Compound(_) => unreachable!("Should not transform compound element"),
            Node {
                name,
                environment: _,
                children: _,
            } => {
                // FIXME: should do the same stuff as in moduleinvocation
                Ok(Element::Compound(vec![Element::ModuleInvocation {
                    name: "raw".to_string(),
                    args: ModuleArguments {
                        positioned: None,
                        named: None,
                    },
                    body: "ok".to_string(),
                    one_line: true,
                }]))
            }
            ModuleInvocation {
                name: module_name,
                args: _,
                body: _,
                one_line: _,
            } => {
                // We find the package responsible for this transform
                let Some((transform, package)) = self.transforms.get(&(module_name.clone(), output_format.clone())) else {
                    return Err(CoreError::MissingTransform(module_name.clone(), output_format.to_string()));
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
                    .args(["transform", module_name, &output_format.to_string()])
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

fn serialize_element(element: &Element, args_info: &Vec<ArgInfo>) -> Result<String, CoreError> {
    use Element::*;
    // [table 10 arg0=20] ge fel om redan finns!
    // [table b=20 a=10 c=30 foo=50]
    match element {
        Data(_) => Ok(serde_json::to_string(element)?),
        Node {
            name: node_name,
            environment: _,
            children,
        } => {
            // FIXME: other node elements should likely support arguments
            let args: HashMap<String, String> = HashMap::new();

            Ok(serde_json::to_string(&json!({
                "name": node_name,
                "arguments": args,
                "children": children,
            }))?)
        }
        ModuleInvocation {
            name: node_name,
            args,
            body,
            one_line,
        } => {
            let mut arguments: HashMap<&String, &String> = HashMap::new();

            // Look up the name of all the positional arguments specified and insert
            // them into the arguments map
            if let Some(positioned) = &args.positioned {
                for (value, arg_info) in positioned.iter().zip(args_info) {
                    let ArgInfo {
                        name,
                        default: _,
                        description: _,
                    } = arg_info;
                    arguments.insert(name, value);
                }
            }

            // Also, add the rest of the key=value paired arguments
            if let Some(named) = &args.named {
                for (name, value) in named {
                    if arguments.contains_key(&name) {
                        return Err(CoreError::RepeatedArgument(node_name.clone(), name.clone()));
                    }
                    arguments.insert(name, value);
                }
            }

            Ok(serde_json::to_string(&json!({
                "name": node_name,
                "arguments": arguments,
                "data": body,
                "inline": one_line,
            }))?)
        }
        Compound(children) => {
            let serialized_children: Vec<String> = children
                .iter()
                .map(|child| serialize_element(child, args_info))
                .collect::<Result<Vec<String>, CoreError>>()?;

            Ok(serde_json::to_string(&json!(serialized_children))?)
        }
    }
}

fn deserialize_compound(input: &str) -> Result<Element, CoreError> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Entry {
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

    fn entry_to_element(entry: Entry) -> Element {
        match entry {
            Entry::ParentNode { name, children } => Element::Node {
                name,
                environment: HashMap::new(),
                children: children
                    .into_iter()
                    .map(|child| entry_to_element(child))
                    .collect(),
            },
            Entry::Module {
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

    let entries: Vec<Entry> = serde_json::from_str(input)?;

    // Convert the parsed entries into real Elements
    let elements: Vec<Element> = entries.into_iter().map(entry_to_element).collect();
    Ok(Element::Compound(elements))
}

fn default_inline() -> bool {
    true
}
