use std::{io::Read, sync::Arc};

use serde::{Deserialize, Serialize};
#[cfg(feature = "native")]
use wasmer::Engine;
use wasmer::{Instance, Module, Store};
use wasmer_wasi::{Pipe, WasiState};

use crate::package::PackageImplementation::Native;
use crate::{error::CoreError, OutputFormat};

/// Transform from a node into another node
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Transform {
    pub from: String,
    pub to: Vec<OutputFormat>,
    pub description: Option<String>,
    pub arguments: Vec<ArgInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct ArgInfo {
    pub name: String,
    pub default: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
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
    /// This function gets a package from its precompiled source. It is similar to Package::new,
    /// but replaces the compilation step by Module::deserialize.
    /// Safety: Notice that the precompiled source bytes
    /// 1. Are going to be deserialized directly into Rust objects.
    /// 2. Contains the function assembly bodies and, if intercepted,
    ///    a malicious actor could inject code into executable
    ///    memory.
    #[cfg(all(feature = "native", feature = "precompile_wasm"))]
    pub(crate) fn new_precompiled(
        precompiled_source: &[u8],
        engine: &Engine,
    ) -> Result<Self, CoreError> {
        let mut store = Store::new(engine);
        // SAFETY: This is safe since we have compiled the sources ourself and has not been
        // tampered with
        let module = unsafe { Module::deserialize(&store, precompiled_source) }?;
        let package_info = Self::read_manifest(&module, &mut store)?;
        Ok(Package {
            info: Arc::new(package_info),
            implementation: PackageImplementation::Wasm(module),
        })
    }

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
