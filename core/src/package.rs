use std::collections::HashMap;
use std::{io::Read, sync::Arc};

use serde::{Deserialize, Serialize, Serializer};
use serde_json::value::Serializer as JsonSerializer;
use serde_json::Value;
#[cfg(feature = "native")]
use wasmer::Engine;
use wasmer::{Instance, Module, Store};
use wasmer_wasi::{Pipe, WasiState};

use crate::package::PackageImplementation::Native;
use crate::{error::CoreError, OutputFormat};

/// Transform from a node into another node
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Transform {
    pub from: String,
    pub to: Vec<OutputFormat>,
    pub description: Option<String>,
    pub arguments: Vec<ArgInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ArgInfo {
    pub name: String,
    pub default: Option<Value>,
    pub description: String,
    #[serde(default = "default_arg_type")]
    pub r#type: ArgType,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArgType {
    #[serde(alias = "string")]
    String,
    #[serde(alias = "int", alias = "integer", alias = "i64")]
    Integer,
    #[serde(
        rename = "Unsigned integer",
        alias = "uint",
        alias = "unsigned_integer",
        alias = "u64"
    )]
    UnsignedInteger,
    #[serde(alias = "float", alias = "number", alias = "f64")]
    Float,
}

fn default_arg_type() -> ArgType {
    ArgType::String
}

impl ArgType {
    pub(crate) fn is_same_type(&self, value: &Value) -> bool {
        match self {
            ArgType::String => value.is_string(),
            ArgType::Integer => value.is_i64(),
            ArgType::UnsignedInteger => value.is_u64(),
            ArgType::Float => value.is_f64(),
        }
    }

    pub(crate) fn try_to_value(&self, value: &str) -> Result<Value, CoreError> {
        match self {
            ArgType::String => Ok(JsonSerializer.serialize_str(value)?),
            ArgType::Integer => {
                let integer = value
                    .parse::<i64>()
                    .map_err(|_| CoreError::ArgumentType("integer", value.to_string()))?;
                Ok(JsonSerializer.serialize_i64(integer)?)
            }
            ArgType::UnsignedInteger => {
                let integer = value
                    .parse::<u64>()
                    .map_err(|_| CoreError::ArgumentType("unsigned integer", value.to_string()))?;
                Ok(JsonSerializer.serialize_u64(integer)?)
            }
            ArgType::Float => {
                let float = value
                    .parse::<f64>()
                    .map_err(|_| CoreError::ArgumentType("float", value.to_string()))?;
                Ok(JsonSerializer.serialize_f64(float)?)
            }
        }
    }
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
        let manifest: PackageInfo = {
            let mut buffer = String::new();
            output.read_to_string(&mut buffer)?;
            serde_json::from_str(&buffer).map_err(|error| CoreError::DeserializationError {
                string: buffer.clone(),
                error,
            })
        }?;

        // Ensure all default values have the correct type
        for transform in &manifest.transforms {
            for argument in &transform.arguments {
                if let Some(default) = argument.default.as_ref() {
                    if !argument.r#type.is_same_type(default) {
                        return Err(CoreError::DefaultArgumentType(
                            argument.name.to_string(),
                            transform.from.to_string(),
                            manifest.name.to_string(),
                            serde_json::to_string(&argument.r#type).unwrap(),
                            default.clone().to_rust_string(),
                        ));
                    }
                }
            }
        }

        Ok(manifest)
    }

    pub fn new_native(info: PackageInfo) -> Result<Self, CoreError> {
        Ok(Package {
            info: Arc::new(info),
            implementation: Native,
        })
    }
}

pub(crate) trait ValueExt {
    fn to_rust_string(self) -> String;
}

impl ValueExt for Value {
    fn to_rust_string(self) -> String {
        match self {
            Value::String(s) => s,
            x => x.to_string(),
        }
    }
}

pub(crate) trait HashMapExt {
    fn map_map(self) -> HashMap<String, String>;
}

impl HashMapExt for HashMap<String, Value> {
    fn map_map(mut self) -> HashMap<String, String> {
        self.drain()
            .map(|(k, v)| (k, ValueExt::to_rust_string(v)))
            .collect()
    }
}
