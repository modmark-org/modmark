use parser::Element;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
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
    #[error("Failed to parse attributes of module '{0}'.")]
    ParseAttribute(String),
}

/// Evaluates a document using the given context
pub fn eval(document: &Element, ctx: &mut Context) -> String {
    todo!()
}

type NodeName = String;

/// Transform from a node into another node
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Transform(NodeName, NodeName);

#[derive(Debug)]
struct ModuleInfo {
    pub name: String,
    pub version: String,
    pub transforms: HashSet<Transform>,
    pub required_attributes: HashSet<String>,
    pub optional_attributes: HashMap<String, String>, //Name to default val
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
        let mut buf = String::new();
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

        // FIXME: populate these other fields after running the parser
        // parse_transforms(raw_transforms_str)
        let module_info = ModuleInfo {
            name,
            version,
            transforms: todo!(),
            required_attributes: todo!(),
            optional_attributes: todo!(),
        };

        Ok(LoadedModule {
            info: Arc::new(module_info),
            wasm_module: module,
        })
    }
}

/// A helper to parse the output from the "transforms" function
/// when loading a module.
/// Transforms are written like this:
/// ```text
/// [foo arg1 arg2 optional_arg=20] -> tex html
/// bar -> html
/// ```
/// FIXME: might be nice to actually use the argument
/// parser used in the `parser` crate to make this a
/// bit more reliable especially when parsing optional args
fn parse_transforms(input: &str) -> Option<()> {
    todo!()
}

pub struct Context {
    transforms: HashMap<Transform, LoadedModule>,
    store: Store,
}

impl Context {
    fn new() -> Self {
        Context {
            transforms: HashMap::new(),
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
        for transform @ Transform(from, to) in &module.info.transforms {
            // Ensure that there are no other modules responsible for this transform.
            if self.transforms.contains_key(transform) {
                return Err(CoreError::OccupiedTransform(
                    from.clone(),
                    to.clone(),
                    module.info.name.clone(),
                ));
            }

            self.transforms.insert(transform.clone(), module.clone());
        }

        Ok(())
    }

    /// Transform a node into another node by using the available transforms
    fn transform(&mut self, elem: Element) -> Result<Element, CoreError> {
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
