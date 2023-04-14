use std::error::Error;

use thiserror::Error;
#[cfg(feature = "native")]
#[allow(unused_imports)]
use wasmer::{CompileError, DeserializeError};
use wasmer::{ExportError, InstantiationError, RuntimeError};
use wasmer_wasi::{WasiError, WasiStateCreationError};

use crate::package::ArgType;
use crate::variables::{VarAccess, VarType};
use parser::ParseError;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("There is already a package named '{0}'.")]
    OccupiedName(String),
    #[error(
        "Could not load package '{2}'. There is another package that transforms '{0}' to '{1}'."
    )]
    OccupiedTransform(String, String, String),
    #[error("Could not load package '{1}'. There is another native transform from '{0}'.")]
    OccupiedNativeTransform(String, String),
    #[cfg(feature = "native")]
    #[error("Compiler error")]
    WasmerCompiler(Box<CompileError>),
    #[cfg(all(feature = "native", feature = "precompile_wasm"))]
    #[error("Error deserializing pre-compiled module")]
    Deserialize(#[from] DeserializeError),
    #[error("No package for transforming node '{0}' to '{1}'.")]
    MissingTransform(String, String),
    #[error("Wasi error '{0}'.")]
    WasiError(Box<WasiError>),
    #[error("Wasmer intstantiation error '{0}'.")]
    WasmerInstantiation(Box<InstantiationError>),
    #[error("Wasi state creation error '{0}'.")]
    WasiStateCreation(Box<WasiStateCreationError>),
    #[error("Wasmer export error '{0}'.")]
    WasmerExport(Box<ExportError>),
    #[error("Wasmer runtime error '{0}'.")]
    WasmerRuntimeError(Box<RuntimeError>),
    #[error("Failed to write/read to or from a package '{0}'.")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse transforms of package '{0}'.")]
    ParseTransforms(String),
    #[error("Package '{0}' contains a transform from '{1}' with overlapping output formats. If 'any' is specified as output format, no other formats should be specified.")]
    OverlappingOutputFormats(String, String),
    #[error("You repeated the argument '{0}' for the '{1}' element.")]
    RepeatedArgument(String, String),
    #[error("Argument '{0}' is missing in element '{1}'.")]
    MissingArgument(String, String),
    #[error("Argument '{0}' is not supported by the element '{1}'.")]
    InvalidArgument(String, String),
    #[error("Json error")]
    JsonError(#[from] serde_json::Error),
    #[error("Failed to deserialize '{string}', got error '{error}'.")]
    DeserializationError {
        string: String,
        error: serde_json::Error,
    },
    #[error("Transform does not terminate")]
    NonTerminatingTransform,
    #[error("Parsing error: {0}.")]
    Parsing(#[from] ParseError),
    #[error("Native call error: Non-module given to package {0} named {1}")]
    NonModuleToNative(String, String),
    #[error("Root element is not a parent, cannot remove __document for playground")]
    RootElementNotParent,
    #[error("Error resolving {0}: {1:?}")]
    Resolve(String, Box<dyn Error + Send>),
    #[error("Duplicate configuration for package '{0}'")]
    DuplicateConfig(String),
    #[error("Unused configurations for package '{0}'")]
    UnusedConfig(String),
    #[error("Config modules are only allowed once at the very top of the document")]
    UnexpectedConfigModule,
    #[error("Error serializing '{0}': {1}")]
    SerializeElement(String, Box<CoreError>),
    #[error("Invalid data type: expected {0} but got '{1}'")]
    ArgumentType(&'static str, String),
    #[error("Invalid enum variant: expected one of {0:?}, but got '{1}'")]
    EnumVariant(Vec<String>, String),
    #[error("Invalid data type for default argument '{argument_name}' for transform '{transform}' in package '{package}', expected type '{expected_type}' but got the value '{given_value}'")]
    DefaultArgumentType {
        argument_name: String,
        transform: String,
        package: String,
        expected_type: String,
        given_value: String,
    },
    #[error("Could not find argument '{argument_name}' which variable accesses depends on in transform '{transform}' in package '{package}' (variable access '{var_access:?}')")]
    ArgumentDependentVariable {
        argument_name: String,
        transform: String,
        package: String,
        var_access: VarAccess,
    },
    #[error("Invalid argument dependent variable type, expected String or Enum variant, got '{argument_type:?}'; argument '{argument_name}' for transform '{transform}' in package '{package}'")]
    ArgumentDependentVariableType {
        argument_type: ArgType,
        argument_name: String,
        transform: String,
        package: String,
    },
    #[error("Attempted to access variable '{variable_name}' using multiple different variable types for transform '{transform}' in '{package}' ")]
    ClashingVariableAccesses {
        variable_name: String,
        transform: String,
        package: String,
    },
    #[error("Attempted to access the variable '{name}' as a '{expected_type}' but the name is already occupied by value of type '{present_type}'.")]
    TypeMismatch {
        name: String,
        expected_type: VarType,
        present_type: VarType,
    },
    #[error("Attempted to redeclare the constant '{0}'.")]
    ConstantRedeclaration(String),
    #[error("Forbidden variable name '{0}'. Only ASCII letters and digits as well as '_' is allowed. The name may also not start with a digit.")]
    ForbiddenVariableName(String),
    #[error("A package request was dropped before resolving")]
    DroppedRequest,
    #[error("Missing standard package named '{0}'")]
    NoSuchStdPackage(String),
    #[error("Could not generate a good schedule; there might be cyclic dependencies")]
    Schedule,
    #[error("Could not flatten structure, internal scheduling error")]
    Flat,
}

impl From<WasiError> for CoreError {
    fn from(value: WasiError) -> Self {
        CoreError::WasiError(Box::new(value))
    }
}

impl From<InstantiationError> for CoreError {
    fn from(value: InstantiationError) -> Self {
        CoreError::WasmerInstantiation(Box::new(value))
    }
}

impl From<WasiStateCreationError> for CoreError {
    fn from(value: WasiStateCreationError) -> Self {
        CoreError::WasiStateCreation(Box::new(value))
    }
}

impl From<ExportError> for CoreError {
    fn from(value: ExportError) -> Self {
        CoreError::WasmerExport(Box::new(value))
    }
}

impl From<RuntimeError> for CoreError {
    fn from(value: RuntimeError) -> Self {
        CoreError::WasmerRuntimeError(Box::new(value))
    }
}

#[cfg(feature = "native")]
impl From<CompileError> for CoreError {
    fn from(value: CompileError) -> Self {
        CoreError::WasmerCompiler(Box::new(value))
    }
}
