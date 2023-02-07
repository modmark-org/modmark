use thiserror::Error;
#[cfg(feature = "native")]
use wasmer::CompileError;
use wasmer::{ExportError, InstantiationError, RuntimeError};
use wasmer_wasi::{WasiError, WasiStateCreationError};

use crate::NodeName;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("There is already a package named '{0}'.")]
    OccupiedName(String),
    #[error(
        "Could not load package '{2}'. There is another package that transforms '{0}' to '{1}'."
    )]
    OccupiedTransform(NodeName, NodeName, String),
    #[cfg(feature = "native")]
    #[error("Compiler error")]
    WasmerCompiler(Box<CompileError>),
    #[error("No module for transforming node '{0}'.")]
    MissingTransform(String),
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
    #[error("Module '{0}' has written invalid UTF-8 to stdout.")]
    InvalidUTF8(String),
    #[error("Failed to parse transforms of package '{0}'.")]
    ParseTransforms(String),
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
