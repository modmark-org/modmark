use std::{
    collections::HashMap,
    io::{Read, Write},
};

use parser::{Element, ModuleArguments};
use serde_json::json;
#[cfg(feature = "native")]
use wasmer::Cranelift;
use wasmer::{Instance, Store};
use wasmer_wasi::{Pipe, WasiState};

use crate::{ArgInfo, CoreError, NodeName, Package, PackageInfo, Transform};

#[derive(Debug)]
pub struct Context {
    packages: HashMap<String, Package>,
    transforms: HashMap<(String, String), (Transform, Package)>,
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
            "/test-module/wasm32-wasi/release/test-module.wasm"
        )))
        .expect("Failed to load test-module module");
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
                args_info: _,
            } = transform;

            for output_format in to {
                // Ensure that there are no other packages responsible for transforming to the same output format.
                if self
                    .transforms
                    .contains_key(&(from.clone(), output_format.clone()))
                {
                    return Err(CoreError::OccupiedTransform(
                        from.clone(),
                        output_format.clone(),
                        pkg.info.name.clone(),
                    ));
                }

                self.transforms.insert(
                    (from.to_string(), output_format.to_string()),
                    (transform.clone(), pkg.clone()),
                );
            }
        }

        Ok(())
    }

    /// Transform a node into another node by using the available transforms
    pub fn transform(
        &mut self,
        from: &Element,
        output_format: &String,
    ) -> Result<Element, CoreError> {
        use Element::*;
        // hittar vilket paket som ansvarar för en element-typ
        // anropar den via wasi och berättar vilket output-format vi förväntar oss
        // får tillbaka en lista som är en kombination av flera andra Element::ModuleInvocations och Element::Data(...)
        // som är färdig-evaluerad och bara innehåller outputformatet

        // Vi skapar en ny variant Element::Compound(Vec<Element>)
        // när man anropar expand(Compound()) om alla innehåller Data kan vi kollapsa den och byta ut noden mot en enda Data()
        match from {
            Data(text) => Ok(Element::ModuleInvocation {
                // FIXME: måste först escapeas till rätt output format, så vi vill nog inte ha någon "data" nod
                // när vi tar över Element utan bara översätta Ast::Text => [text]...
                name: "output".to_string(),
                args: ModuleArguments {
                    positioned: None,
                    named: None,
                },
                body: text.clone(),
                one_line: true,
            }),
            Node {
                name,
                environment,
                children,
            } => todo!(),
            ModuleInvocation {
                name: module_name,
                args,
                body,
                one_line,
            } => {
                if module_name == "output" {
                    unreachable!("Transform should never be called on an output module")
                }

                // We find the package responsible for this transform
                let Some((transform, package)) = self.transforms.get(&(module_name.clone(), output_format.clone())) else {
                    return Err(CoreError::MissingTransform(module_name.clone(), output_format.clone()));
                };

                let mut input = Pipe::new();
                let mut output = Pipe::new();
                write!(
                    &mut input,
                    "{}",
                    serialize_element(from, &transform.args_info)?
                )?;

                let wasi_env = WasiState::new("")
                    .stdin(Box::new(input))
                    .stdout(Box::new(output.clone()))
                    .args(["transform", module_name])
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
                    buffer.trim().to_string()
                };

                // FIXME: right now, everything is wrapped inside of a ducument,
                // instead we should use seperate parsers for inline and block depending on the variable one_line
                Ok(parser::parse(&result))
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
    }
}

#[test]
fn test_serialize() {
    let mut ctx = Context::default();

    dbg!(serialize_element(
        &Element::ModuleInvocation {
            name: "table".to_string(),
            args: ModuleArguments {
                positioned: Some(vec!["foo".to_string(), "bar".to_string()]),
                named: Some({
                    let mut map = HashMap::new();
                    map.insert("baz".to_string(), "100".to_string());
                    map
                }),
            },
            body: "testing".to_string(),
            one_line: false,
        },
        &vec![]
    ));
}

#[test]
fn test_tranform() {
    let mut ctx = Context::default();
    let output_format = "html".to_string();
    ctx.transform(
        &Element::ModuleInvocation {
            name: "table".to_string(),
            args: ModuleArguments {
                positioned: Some(vec!["foo".to_string(), "bar".to_string()]),
                named: Some({
                    let mut map = HashMap::new();
                    map.insert("baz".to_string(), "100".to_string());
                    map
                }),
            },
            body: "testing".to_string(),
            one_line: false,
        },
        &output_format,
    )
    .unwrap();
}
