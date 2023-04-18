use std::collections::HashMap;
use std::{io::Read, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "native")]
use wasmer::Engine;
use wasmer::{Instance, Module, Store};
use wasmer_wasi::{Pipe, WasiState};

use crate::package::PackageImplementation::Native;
use crate::variables::VarAccess;
use crate::{error::CoreError, OutputFormat};

/// Transform from a node into another node
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Transform {
    pub from: String,
    pub to: Vec<OutputFormat>,
    pub description: Option<String>,
    pub arguments: Vec<ArgInfo>,
    #[serde(default)]
    pub variables: HashMap<String, VarAccess>,
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
        // Ensure all mentioned argument-dependent variables has corresponding arguments
        for transform in &self.transforms {
            for (var, access) in &transform.variables {
                if let Some(arg_name) = var.strip_prefix('$') {
                    // Check if the argument exist
                    if let Some(arg_info) =
                        transform.arguments.iter().find(|arg| arg.name == arg_name)
                    {
                        // If we find it, it must be of the type String or Enum
                        if arg_info.r#type != ArgType::Primitive(PrimitiveArgType::String)
                            && !matches!(arg_info.r#type, ArgType::Enum(_))
                        {
                            return Err(CoreError::ArgumentDependentVariableType {
                                argument_type: arg_info.r#type.clone(),
                                argument_name: arg_name.to_string(),
                                transform: transform.from.to_string(),
                                package: self.name.to_string(),
                            });
                        }
                    } else {
                        // If not, we are missing that argument and the manifest is invalid
                        return Err(CoreError::ArgumentDependentVariable {
                            argument_name: arg_name.to_string(),
                            transform: transform.from.to_string(),
                            package: self.name.to_string(),
                            var_access: *access,
                        });
                    }
                }
            }
        }

        // Ensure package does not specify other output formats when "any" is specified
        for transform in &self.transforms {
            if transform.to.contains(&OutputFormat::Any) && transform.to.len() > 1 {
                return Err(CoreError::OverlappingOutputFormats(
                    self.name.to_string(),
                    transform.from.clone(),
                ));
            }
        }

        Ok(())
    }
}

impl Transform {
    pub(crate) fn has_argument_dependent_variable(&self) -> bool {
        self.variables.keys().any(|key| key.starts_with('$'))
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

// This is more or less just a wrapper to simplify writing enums like `type: [true, false]` without
// needing to tag it. The distinction between "ArgType" and "PrimitiveArgType" is that a
// primitive arg type is one rust type, as simple as that, while an ArgType may be an enum which
// requires additional validation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ArgType {
    Enum(Vec<String>),
    Primitive(PrimitiveArgType),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimitiveArgType {
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
    ArgType::Primitive(PrimitiveArgType::String)
}

impl From<PrimitiveArgType> for ArgType {
    fn from(value: PrimitiveArgType) -> Self {
        Self::Primitive(value)
    }
}

impl ArgType {
    pub(crate) fn can_be_parsed_from(&self, value: &Value) -> bool {
        match &self {
            ArgType::Enum(vs) => value
                .as_str()
                .map_or(false, |x| vs.contains(&x.to_string())),
            ArgType::Primitive(t) => t.can_be_parsed_from(value),
        }
    }

    pub(crate) fn try_from_value(&self, value: &Value) -> Result<ArgValue, CoreError> {
        match self {
            ArgType::Enum(values) => value
                .as_str()
                .map(ToString::to_string)
                .filter(|s| values.contains(s))
                .map(ArgValue::EnumVariant)
                .ok_or(CoreError::EnumVariant(values.clone(), value.to_string())),
            ArgType::Primitive(t) => t.try_from_value(value),
        }
    }

    pub(crate) fn try_from_str(&self, value: &str) -> Result<ArgValue, CoreError> {
        match self {
            ArgType::Enum(values) => {
                let string = value.to_string();
                if values.contains(&string) {
                    Ok(ArgValue::EnumVariant(string))
                } else {
                    Err(CoreError::EnumVariant(values.clone(), value.to_string()))
                }
            }
            ArgType::Primitive(t) => t.try_from_str(value),
        }
    }
}

impl PrimitiveArgType {
    pub(crate) fn can_be_parsed_from(&self, value: &Value) -> bool {
        match self {
            PrimitiveArgType::String => value.is_string(),
            PrimitiveArgType::Integer => value.is_i64(),
            PrimitiveArgType::UnsignedInteger => value.is_u64(),
            PrimitiveArgType::Float => value.is_f64(),
        }
    }

    pub(crate) fn try_from_value(&self, value: &Value) -> Result<ArgValue, CoreError> {
        match self {
            PrimitiveArgType::String => value
                .as_str()
                .map(|s| ArgValue::String(s.to_string()))
                .ok_or(CoreError::ArgumentType("string", value.to_string())),
            PrimitiveArgType::Integer => value
                .as_i64()
                .map(ArgValue::Integer)
                .ok_or(CoreError::ArgumentType("integer", value.to_string())),
            PrimitiveArgType::UnsignedInteger => value
                .as_u64()
                .map(ArgValue::UnsignedInteger)
                .ok_or(CoreError::ArgumentType(
                    "unsigned integer",
                    value.to_string(),
                )),
            PrimitiveArgType::Float => value
                .as_f64()
                .map(ArgValue::Float)
                .ok_or(CoreError::ArgumentType("float", value.to_string())),
        }
    }

    pub(crate) fn try_from_str(&self, value: &str) -> Result<ArgValue, CoreError> {
        match self {
            PrimitiveArgType::String => Ok(ArgValue::String(value.to_string())),
            PrimitiveArgType::Integer => {
                let integer = value
                    .parse::<i64>()
                    .map_err(|_| CoreError::ArgumentType("integer", value.to_string()))?;
                Ok(ArgValue::Integer(integer))
            }
            PrimitiveArgType::UnsignedInteger => {
                let integer = value
                    .parse::<u64>()
                    .map_err(|_| CoreError::ArgumentType("unsigned integer", value.to_string()))?;
                Ok(ArgValue::UnsignedInteger(integer))
            }
            PrimitiveArgType::Float => {
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
    EnumVariant(String),
}

impl From<ArgValue> for Value {
    fn from(value: ArgValue) -> Self {
        match value {
            ArgValue::String(s) | ArgValue::EnumVariant(s) => Value::from(s),
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
            ArgValue::String(s) | ArgValue::EnumVariant(s) => s,
            ArgValue::Integer(i) => format!("{i}"),
            ArgValue::UnsignedInteger(ui) => format!("{ui}"),
            ArgValue::Float(f) => format!("{f}"),
        }
    }
}

impl ArgValue {
    pub fn get_type(&self) -> ArgType {
        match &self {
            ArgValue::String(_) => PrimitiveArgType::String.into(),
            ArgValue::Integer(_) => PrimitiveArgType::Integer.into(),
            ArgValue::UnsignedInteger(_) => PrimitiveArgType::UnsignedInteger.into(),
            ArgValue::Float(_) => PrimitiveArgType::Float.into(),
            // We have variant-erasure but I think that's OK
            ArgValue::EnumVariant(_) => ArgType::Enum(vec![]),
        }
    }

    pub fn get_enum_variant(self) -> Option<String> {
        if let ArgValue::EnumVariant(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
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

    pub fn get_integer(self) -> Option<i64> {
        if let ArgValue::Integer(i) = self {
            Some(i)
        } else {
            None
        }
    }

    pub fn get_unsigned_integer(self) -> Option<u64> {
        if let ArgValue::UnsignedInteger(i) = self {
            Some(i)
        } else {
            None
        }
    }

    pub fn get_float(self) -> Option<f64> {
        if let ArgValue::Float(f) = self {
            Some(f)
        } else {
            None
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
    pub fn new(
        wasm_source: &[u8],
        #[cfg(feature = "native")] engine: &Engine,
    ) -> Result<Self, CoreError> {
        #[cfg(feature = "native")]
        let mut store = Store::new(engine);
        #[cfg(feature = "native")]
        let module = Module::from_binary(engine, wasm_source)?;

        #[cfg(feature = "web")]
        let mut store = Store::new();
        #[cfg(feature = "web")]
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
