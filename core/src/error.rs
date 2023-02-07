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
    WasmerCompiler(#[from] CompileError),
    #[error("No module for transforming node '{0}'.")]
    MissingTransform(String),
    #[error("Wasi error '{0}'.")]
    WasiError(#[from] WasiError),
    #[error("Wasmer intstantiation error '{0}'.")]
    WasmerInstantiation(#[from] InstantiationError),
    #[error("Wasi state creation error '{0}'.")]
    WasiStateCreation(#[from] WasiStateCreationError),
    #[error("Wasmer export error '{0}'.")]
    WasmerExport(#[from] ExportError),
    #[error("Wasmer runtime error '{0}'.")]
    WasmerRuntimeError(#[from] RuntimeError),
    #[error("Module '{0}' has written invalid UTF-8 to stdout.")]
    InvalidUTF8(String),
    #[error("Failed to parse transforms of package '{0}'.")]
    ParseTransforms(String),
}
