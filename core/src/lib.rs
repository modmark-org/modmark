use parser::Element;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use thiserror::Error;
use wasmer::{
    CompileError, Cranelift, ExportError, Instance, InstantiationError, Module, RuntimeError, Store,
};
use wasmer_wasi::{Pipe, WasiError, WasiState, WasiStateCreationError};

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("There is already a module named '{0}'.")]
    OccupiedName(String),
    #[error(
        "Could not load module '{2}'. There is another module that transforms '{0}' to '{1}'."
    )]
    OccupiedTransform(NodeName, NodeName, String),
    #[error("Compiler error")]
    WasmerCompiler(CompileError),
    #[error("No module for transforming node '{0}'.")]
    MissingTransform(String),
    #[error("Wasi error '{0}'.")]
    WasiError(WasiError),
    #[error("Wasmer intstantiation error '{0}'.")]
    WasmerInstantiation(InstantiationError),
    #[error("Wasi state creation error '{0}'.")]
    WasiStateCreation(WasiStateCreationError),
    #[error("Wasmer export error '{0}'.")]
    WasmerExport(ExportError),
    #[error("Wasmer runtime error '{0}'.")]
    WasmerRuntimeError(RuntimeError),
    #[error("Module '{0}' has written invalid UTF-8 to stdout.")]
    InvalidUTF8(String),
    #[error("Failed to parse transforms of module '{0}'.")]
    ParseTransforms(String),
}

/// Evaluates a document using the given context
pub fn eval(_document: &Element, _ctx: &mut Context) -> String {
    "TODO".to_string()
}

type NodeName = String;

/// Transform from a node into another node
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Transform {
    from: NodeName,
    to: NodeName,
    arguments: Vec<Arg>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Arg {
    name: String,
    default: Option<String>,
    description: String,
}

#[derive(Debug)]
struct ModuleInfo {
    pub name: String,
    pub version: String,
    pub transforms: Vec<Transform>,
}

#[derive(Debug, Clone)]
struct LoadedModule {
    pub info: Arc<ModuleInfo>,
    pub wasm_module: Module,
}

impl LoadedModule {
    /// Read the binary data from a `.wasm` file and create a LoadedModule
    /// containing info about the module as well as the compiled wasm source.
    fn new(wasm_source: &[u8], store: &mut Store) -> Result<Self, CoreError> {
        // Compile the module and store it
        let module = Module::from_binary(store, wasm_source).map_err(CoreError::WasmerCompiler)?;

        let mut input = Pipe::new();
        let mut output = Pipe::new();

        let wasi_env = WasiState::new("")
            .stdin(Box::new(input.clone()))
            .stdout(Box::new(output.clone()))
            .finalize(store)
            .map_err(CoreError::WasiStateCreation)?;

        let import_object = wasi_env
            .import_object(store, &module)
            .map_err(CoreError::WasiError)?;
        let instance = Instance::new(store, &module, &import_object)
            .map_err(CoreError::WasmerInstantiation)?;

        // Attach the memory export
        let memory = instance
            .exports
            .get_memory("memory")
            .map_err(CoreError::WasmerExport)?;
        wasi_env.data_mut(store).set_memory(memory.clone());

        // Retrieve name from module
        // Call the `name` function
        let name_fn = instance
            .exports
            .get_function("name")
            .map_err(CoreError::WasmerExport)?;
        name_fn
            .call(store, &[])
            .map_err(CoreError::WasmerRuntimeError)?;

        // Read the name from stdout
        let name = {
            let mut buffer = String::new();
            output
                .read_to_string(&mut buffer)
                .map_err(|_| CoreError::InvalidUTF8("unkown (when reading name)".to_string()))?;
            buffer
        };

        // Retrieve version from module
        // Call the `version` function
        let version_fn = instance
            .exports
            .get_function("version")
            .map_err(CoreError::WasmerExport)?;
        version_fn
            .call(store, &[])
            .map_err(CoreError::WasmerRuntimeError)?;

        // Read the version from stdout
        let version = {
            let mut buffer = String::new();
            output
                .read_to_string(&mut buffer)
                .map_err(|_| CoreError::InvalidUTF8(name.clone()))?;
            buffer
        };

        // Retrieve transform capabilities of module
        let transforms_fn = instance
            .exports
            .get_function("transforms")
            .map_err(CoreError::WasmerExport)?;
        transforms_fn
            .call(store, &[])
            .map_err(CoreError::WasmerRuntimeError)?;

        let raw_transforms_str = {
            let mut buffer = String::new();
            output
                .read_to_string(&mut buffer)
                .map_err(|_| CoreError::InvalidUTF8(name.clone()))?;
            buffer
        };

        let Some(transforms) = parse_transforms(&raw_transforms_str) else {
            return Err(CoreError::ParseTransforms(name))
        };

        let module_info = ModuleInfo {
            name,
            version,
            transforms,
        };

        Ok(LoadedModule {
            info: Arc::new(module_info),
            wasm_module: module,
        })
    }
}

/// A helper to parse the output from the "transforms" function
/// when loading a module.
fn parse_transforms(input: &str) -> Option<Vec<Transform>> {
    let mut transforms = Vec::new();
    let mut lines = input.lines();

    while let Some(line) = lines.next() {
        if line.trim().is_empty() {
            continue;
        }
        // [foo] -> html tex
        let (raw_name, raw_outputs) = line.split_once("->")?;
        let name = raw_name.trim();
        let outputs: Vec<String> = raw_outputs
            .split_whitespace()
            .map(|output| output.trim().to_string())
            .collect();

        // parse the following lines of arguments up until
        // the next blank line
        // foo = 20 - A description
        let mut arguments = Vec::new();

        while let Some(line) = lines.next() {
            if line.trim().is_empty() {
                break;
            }
            if let Some(arg) = parse_arg(line) {
                arguments.push(arg);
            };
        }

        // Add a tranform entry for each output
        for output in outputs {
            transforms.push(Transform {
                from: name.to_string(),
                to: output,
                arguments: arguments.clone(),
            });
        }
    }

    Some(transforms)
}

/// Parse an argument written like this
/// name = "optional default value" - Description
/// FIXME: this parser breaks on the following input
/// foo ="-" - description
fn parse_arg(input: &str) -> Option<Arg> {
    let (lhs, description) = input.split_once('-')?;
    let name = lhs.split_whitespace().take(1).collect();
    let maybe_default = lhs.split_whitespace().skip(1).collect::<String>();
    let default = maybe_default.strip_prefix('=');

    Some(Arg {
        name,
        description: description.trim().to_string(),
        default: default.map(|s| s.trim().to_string()),
    })
}

pub struct Context {
    transforms: HashMap<Transform, LoadedModule>,
    all_transforms_for_node: HashMap<NodeName, Vec<(Transform, LoadedModule)>>,
    store: Store,
}

impl Context {
    fn new() -> Self {
        Context {
            transforms: HashMap::new(),
            all_transforms_for_node: HashMap::new(),
            store: Store::new(Cranelift::default()),
        }
    }

    fn load_default_modules(&mut self) {
        self.load_module(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/bold/wasm32-wasi/release/bold.wasm"
        )))
        .expect("Failed to load bold module");

        self.load_module(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/test-module/wasm32-wasi/release/test-module.wasm"
        )))
        .expect("Failed to load test-module module");
    }

    fn load_module(&mut self, wasm_source: &[u8]) -> Result<(), CoreError> {
        let module = LoadedModule::new(wasm_source, &mut self.store)?;

        // Go through all transforms that the module supports and add them
        // to the Context.
        for transform in &module.info.transforms {
            let Transform {
                from,
                to,
                arguments: _,
            } = transform;

            // Ensure that there are no other modules responsible for this transform.
            if self.transforms.contains_key(transform) {
                return Err(CoreError::OccupiedTransform(
                    from.clone(),
                    to.clone(),
                    module.info.name.clone(),
                ));
            }

            // Insert the transform into the context
            self.transforms.insert(transform.clone(), module.clone());

            // We also want to update another hashtable to get quick lookups when
            // trying to transform a given node
            if let Some(transforms) = self.all_transforms_for_node.get_mut(from) {
                transforms.push((transform.clone(), module.clone()));
            } else {
                self.all_transforms_for_node
                    .insert(from.clone(), vec![(transform.clone(), module.clone())]);
            }
        }

        Ok(())
    }

    /// Transform a node into another node by using the available transforms
    fn transform(&mut self, _elem: Element) -> Result<Element, CoreError> {
        // FIXME: implement this
        todo!()
    }
}

impl Default for Context {
    /// A Context with all default modules lodaded
    fn default() -> Self {
        let mut ctx = Self::new();
        ctx.load_default_modules();
        ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_argument_test() {
        let s = "x = 30 - The y position";
        assert_eq!(
            parse_arg(s),
            Some(Arg {
                name: "x".to_string(),
                default: Some("30".to_string()),
                description: "The y position".to_string(),
            })
        );
    }

    #[test]
    fn parse_transforms_test() {
        let s = r#"[code] -> latex
                arg1 - This is a required positional
                ident = 4 - The amount of spaces to indent the block

                foo -> bar

                baz -> html
                a - Description for a
                b = 1 - Description for b
                c = 2 - Description for "#;

        assert_eq!(parse_transforms(s).is_some(), true);
    }
}
