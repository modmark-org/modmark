use parser::ParseError;
use std::error::Error;
use thiserror::Error;
#[cfg(feature = "native")]
#[allow(unused_imports)]
use wasmer::{CompileError, DeserializeError};
use wasmer::{ExportError, InstantiationError, RuntimeError};
use wasmer_wasi::{WasiError, WasiStateCreationError};

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
    #[error("DenyAllResolver is used; resolving of packages disallowed")]
    DenyAllResolver,
    #[error("Error resolving {0}: {1:?}")]
    Resolve(String, Box<dyn Error>),
    #[error("Duplicate configurations for packages: '{0:?}'")]
    DuplicateConfigs(Vec<String>)
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
