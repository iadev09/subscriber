use std::env::VarError;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::{env, io};

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid UTF-8 output")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Environment variable not set: {0}")]
    EnvError(#[from] VarError),

    #[error("Path not found {0}")]
    PathNotFound(String)
}

use crate::ctx::utils::is_running_under_systemd;

pub async fn init_log() -> Result<(), Error> {
    match get_log_path()? {
        Some(log_path) => log_to_file(log_path)?,
        None => console_logger()
    }
    Ok(())
}

fn get_log_path() -> Result<Option<String>, Error> {
    if is_running_under_systemd() {
        let log_dir = env::var("LOGS_DIRECTORY")
            .or_else(|_| env::var("LOGS_DIR"))
            .unwrap_or_else(|_| "./storage/logs".to_string());

        fs::create_dir_all(&log_dir)?;

        let log_path = PathBuf::from(&log_dir)
            .join("app.log")
            .to_str()
            .ok_or(Error::PathNotFound(log_dir))?
            .to_string();

        Ok(Some(log_path))
    } else {
        Ok(None)
    }
}

fn log_to_file(log_path: String) -> Result<(), Error> {
    let file = OpenOptions::new().create(true).append(true).open(&log_path)?;

    let file_writer = BoxMakeWriter::new(file);

    tracing_subscriber::fmt()
        .with_writer(file_writer)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();

    Ok(())
}

fn console_logger() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();
}
