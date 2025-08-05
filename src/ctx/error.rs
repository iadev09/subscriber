use std::env::VarError;
use std::io;

use serde_json::Error as JsonError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Json error: {0}")]
    JsonError(#[from] JsonError),

    #[error("Invalid UTF-8 output")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Environment variable not set: {0}")]
    EnvError(#[from] VarError),

    #[error("Unexpected error: {0}")]
    Unexpected(String)
}

impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Error::Unexpected(error.to_string())
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Error::Unexpected(error)
    }
}
