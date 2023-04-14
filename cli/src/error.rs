use modmark_core::CoreError;
use parser::ParseError;
use std::io;
use thiserror::Error;
use tokio::task::JoinError;

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

    #[error("Cannot infer output format, please specify --format")]
    UnknownOutputFormat,

    #[error("Reqwest error '{0}'")]
    Reqwest(#[from] reqwest::Error),

    #[error("Tokio join error '{0}'")]
    Join(#[from] JoinError),

    #[error("Serde error '{0}'")]
    Serde(#[from] serde_json::Error),

    #[error("Could not create cache path")]
    Cache,

    #[error("Could not get catalog source")]
    Catalog,

    #[error("Could not find local path to '{0}'")]
    Local(String),

    #[error("Could not download package: Error code '{0}'")]
    Get(String),

    #[error("There are no free ports for the live preview to use")]
    NoFreePorts,

    #[error("Second argument OUTPUT_FILE missing. You may only omit this when compiling to html and using the live preview.")]
    MissingOutputFile,
}
