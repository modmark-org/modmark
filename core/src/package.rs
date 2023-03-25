use std::{io::Read, sync::Arc};

use serde::{Deserialize, Serialize};
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

impl ArgInfo {
    fn verify_default(&self) -> bool {
        self.default
            .as_ref()
            .map(|d| self.r#type.can_be_parsed_from(d))
            .unwrap_or(true)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub transforms: Vec<Transform>,
}

impl PackageInfo {
    fn verify(&self) -> Result<(), CoreError> {
        // Ensure all default values have the correct type
        for transform in &self.transforms {
            for argument in &transform.arguments {
                if !argument.verify_default() {
                    return Err(CoreError::DefaultArgumentType {
                        argument_name: argument.name.to_string(),
                        transform: transform.from.to_string(),
                        package: self.name.to_string(),
                        expected_type: serde_json::to_string(&argument.r#type).unwrap(),
                        given_value: argument
                            .default
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default(),
                    });
                }
            }
        }
        Ok(())
    }
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
    pub(crate) fn can_be_parsed_from(&self, value: &Value) -> bool {
        match self {
            ArgType::String => value.is_string(),
            ArgType::Integer => value.is_i64(),
            ArgType::UnsignedInteger => value.is_u64(),
            ArgType::Float => value.is_f64(),
        }
    }

    pub(crate) fn is_same_type(&self, value: &ArgValue) -> bool {
        self == &value.get_type()
    }

    pub(crate) fn try_from_value(&self, value: &Value) -> Result<ArgValue, CoreError> {
        match self {
            ArgType::String => value
                .as_str()
                .map(|s| ArgValue::String(s.to_string()))
                .ok_or(CoreError::ArgumentType("string", value.to_string())),
            ArgType::Integer => value
                .as_i64()
                .map(ArgValue::Integer)
                .ok_or(CoreError::ArgumentType("integer", value.to_string())),
            ArgType::UnsignedInteger => {
                value
                    .as_u64()
                    .map(ArgValue::UnsignedInteger)
                    .ok_or(CoreError::ArgumentType(
                        "unsigned integer",
                        value.to_string(),
                    ))
            }
            ArgType::Float => value
                .as_f64()
                .map(ArgValue::Float)
                .ok_or(CoreError::ArgumentType("float", value.to_string())),
        }
    }

    pub(crate) fn try_from_str(&self, value: &str) -> Result<ArgValue, CoreError> {
        match self {
            ArgType::String => Ok(ArgValue::String(value.to_string())),
            ArgType::Integer => {
                let integer = value
                    .parse::<i64>()
                    .map_err(|_| CoreError::ArgumentType("integer", value.to_string()))?;
                Ok(ArgValue::Integer(integer))
            }
            ArgType::UnsignedInteger => {
                let integer = value
                    .parse::<u64>()
                    .map_err(|_| CoreError::ArgumentType("unsigned integer", value.to_string()))?;
                Ok(ArgValue::UnsignedInteger(integer))
            }
            ArgType::Float => {
                let float = value
                    .parse::<f64>()
                    .map_err(|_| CoreError::ArgumentType("float", value.to_string()))?;
                Ok(ArgValue::Float(float))
            }
        }
    }
}

// Note: do NOT deserialize this since we can impossibly know what data type the JSON should be in,
// and there is a possible loss of information when deserializing
#[derive(Clone, Debug, PartialEq)]
pub enum ArgValue {
    String(String),
    Integer(i64),
    UnsignedInteger(u64),
    Float(f64),
}

impl From<ArgValue> for Value {
    fn from(value: ArgValue) -> Self {
        match value {
            ArgValue::String(s) => Value::from(s),
            ArgValue::Integer(i) => Value::from(i),
            ArgValue::UnsignedInteger(ui) => Value::from(ui),
            ArgValue::Float(f) => Value::from(f),
        }
    }
}

// We implement Into<String> since we want to consume the value
impl From<ArgValue> for String {
    fn from(value: ArgValue) -> Self {
        match value {
            ArgValue::String(s) => s,
            ArgValue::Integer(i) => format!("{i}"),
            ArgValue::UnsignedInteger(ui) => format!("{ui}"),
            ArgValue::Float(f) => format!("{f}"),
        }
    }
}

impl ArgValue {
    pub fn get_type(&self) -> ArgType {
        match &self {
            ArgValue::String(_) => ArgType::String,
            ArgValue::Integer(_) => ArgType::Integer,
            ArgValue::UnsignedInteger(_) => ArgType::UnsignedInteger,
            ArgValue::Float(_) => ArgType::Float,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        if let ArgValue::String(s) = &self {
            Some(s)
        } else {
            None
        }
    }

    pub fn get_string(self) -> Option<String> {
        if let ArgValue::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn unwrap_string(self) -> String {
        self.get_string().unwrap()
    }

    pub fn get_integer(self) -> Option<i64> {
        if let ArgValue::Integer(i) = self {
            Some(i)
        } else {
            None
        }
    }

    pub fn unwrap_integer(self) -> i64 {
        self.get_integer().unwrap()
    }

    pub fn get_unsigned_integer(self) -> Option<u64> {
        if let ArgValue::UnsignedInteger(i) = self {
            Some(i)
        } else {
            None
        }
    }

    pub fn unwrap_unsigned_integer(self) -> u64 {
        self.get_unsigned_integer().unwrap()
    }

    pub fn get_float(self) -> Option<f64> {
        if let ArgValue::Float(f) = self {
            Some(f)
        } else {
            None
        }
    }

    pub fn unwrap_float(self) -> f64 {
        self.get_float().unwrap()
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

        // Verify that the manifest is valid
        manifest.verify()?;

        Ok(manifest)
    }

    pub fn new_native(info: PackageInfo) -> Result<Self, CoreError> {
        Ok(Package {
            info: Arc::new(info),
            implementation: Native,
        })
    }
}
