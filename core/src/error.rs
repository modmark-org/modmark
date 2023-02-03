use thiserror::Error;
use wasmer::{CompileError, ExportError, InstantiationError, RuntimeError};
use wasmer_wasi::{WasiError, WasiStateCreationError};

use crate::NodeName;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("There is already a module named '{0}'.")]
    OccupiedName(String),
    #[error(
        "Could not load module '{2}'. There is another module that transforms '{0}' to '{1}'."
    )]
    OccupiedTransform(NodeName, NodeName, String),
    #[error("Compiler error")]
    WasmerCompiler(CompileError),
    #[error("No module for transforming node '{0}'.")]
    MissingTransform(String),
    #[error("Wasi error '{0}'.")]
    WasiError(WasiError),
    #[error("Wasmer intstantiation error '{0}'.")]
    WasmerInstantiation(InstantiationError),
    #[error("Wasi state creation error '{0}'.")]
    WasiStateCreation(WasiStateCreationError),
    #[error("Wasmer export error '{0}'.")]
    WasmerExport(ExportError),
    #[error("Wasmer runtime error '{0}'.")]
    WasmerRuntimeError(RuntimeError),
    #[error("Module '{0}' has written invalid UTF-8 to stdout.")]
    InvalidUTF8(String),
    #[error("Failed to parse transforms of module '{0}'.")]
    ParseTransforms(String),
}
