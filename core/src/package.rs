use std::{io::Read, sync::Arc};

use serde::Deserialize;
#[cfg(feature = "native")]
use wasmer::Engine;
use wasmer::{Instance, Module, Store};
use wasmer_wasi::{Pipe, WasiState};

use crate::package::PackageImplementation::Native;
use crate::{error::CoreError, OutputFormat};

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

/// Implements PartialEq for PackageImplementation in a way where two
/// `PackageImplementation::Native` gives `true` but any other combination gives `false`
impl PartialEq for PackageImplementation {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Native => match other {
                Native => true,
                PackageImplementation::Wasm(_) => false,
            },
            PackageImplementation::Wasm(_) => false,
        }
    }
}

impl Package {
    /// Read the binary data from a `.wasm` file and create a Package
    /// containing info about the package as well as the compiled wasm source module.
    #[cfg(feature = "native")]
    pub fn new(wasm_source: &[u8], engine: &Engine) -> Result<Self, CoreError> {
        let module = Module::from_binary(engine, wasm_source)?;
        let mut store = Store::new(engine);
        let package_info = Self::read_manifest(&module, &mut store)?;
        Ok(Package {
            info: Arc::new(package_info),
            implementation: PackageImplementation::Wasm(module),
        })
    }

    /// Read the binary data from a `.wasm` file and create a Package
    /// containing info about the package as well as the compiled wasm source module.
    #[cfg(feature = "web")]
    pub fn new(wasm_source: &[u8]) -> Result<Self, CoreError> {
        // Looking at the code found in the wasmer::js::module it looks like
        // this store never actually will be tied to the Module so it should be fine
        // to create a "dummy" store like this and then later create a new store each
        // time we create a new instance.
        let mut store = Store::new();
        let module =
            Module::from_binary(&store, wasm_source).expect("Failed to create wasm module");

        let package_info = Self::read_manifest(&module, &mut store)?;
        Ok(Package {
            info: Arc::new(package_info),
            implementation: PackageImplementation::Wasm(module),
        })
    }

    fn read_manifest(module: &Module, store: &mut Store) -> Result<PackageInfo, CoreError> {
        let input = Pipe::new();
        let mut output = Pipe::new();

        let wasi_env = WasiState::new("")
            .stdin(Box::new(input))
            .stdout(Box::new(output.clone()))
            .arg("manifest")
            .finalize(store)?;

        let import_object = wasi_env.import_object(store, module)?;
        let instance = Instance::new(store, module, &import_object)?;

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
            serde_json::from_str(&buffer).map_err(|error| CoreError::DeserializationError {
                string: buffer.clone(),
                error,
            })
        };

        manifest
    }

    pub fn new_native(info: PackageInfo) -> Result<Self, CoreError> {
        Ok(Package {
            info: Arc::new(info),
            implementation: Native,
        })
    }
}
