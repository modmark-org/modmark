use core::CoreError;
use parser::ParseError;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Notify error '{0}'")]
    Notify(#[from] notify::Error),

    #[error("IO error '{0}'")]
    Io(#[from] io::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("Core error: {0}")]
    Core(#[from] CoreError),
}
