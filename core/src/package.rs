use crate::package::PackageImplementation::Native;
use crate::{error::CoreError, OutputFormat};
use serde::Deserialize;
use std::{io::Read, sync::Arc};
use wasmer::{Instance, Module, Store};
use wasmer_wasi::{Pipe, WasiState};

/// Transform from a node into another node
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct Transform {
    pub from: String,
    pub to: Vec<OutputFormat>,
    pub arguments: Vec<ArgInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct ArgInfo {
    pub name: String,
    pub default: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub transforms: Vec<Transform>,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub info: Arc<PackageInfo>,
    pub implementation: PackageImplementation,
}

#[derive(Debug, Clone)]
pub enum PackageImplementation {
    Wasm(Module),
    Native,
}

impl Package {
    /// Read the binary data from a `.wasm` file and create a Package
    /// containing info about the package as well as the compiled wasm source module.
    pub fn new(wasm_source: &[u8], store: &mut Store) -> Result<Self, CoreError> {
        // Compile the module and store it
        #[cfg(feature = "native")]
        let module = Module::from_binary(store, wasm_source)?;

        #[cfg(feature = "web")]
        let module = Module::from_binary(store, wasm_source).expect("Web wasm compiler error");

        let input = Pipe::new();
        let mut output = Pipe::new();

        let wasi_env = WasiState::new("")
            .stdin(Box::new(input))
            .stdout(Box::new(output.clone()))
            .arg("manifest")
            .finalize(store)?;

        let import_object = wasi_env.import_object(store, &module)?;
        let instance = Instance::new(store, &module, &import_object)?;

        // Attach the memory export
        let memory = instance.exports.get_memory("memory")?;
        wasi_env.data_mut(store).set_memory(memory.clone());

        // Retrieve manifest of package
        let manifest = instance.exports.get_function("_start")?;
        manifest.call(store, &[])?;

        // Read package info from stdin
        let manifest = {
            let mut buffer = String::new();
            output.read_to_string(&mut buffer)?;
            serde_json::from_str(&buffer)?
        };

        Ok(Package {
            info: Arc::new(manifest),
            implementation: PackageImplementation::Wasm(module),
        })
    }

    pub fn new_native(info: PackageInfo) -> Result<Self, CoreError> {
        Ok(Package {
            info: Arc::new(info),
            implementation: Native,
        })
    }
}
