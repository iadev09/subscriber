use std::error::Error as StdError;
use std::io;

use redis::RedisError;
use serde_json::Error as JsonError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Json error: {0}")]
    Json(#[from] JsonError),

    #[error("Redis message stream ended (None)")]
    Disconnected,

    #[error("Unhandled Redis error: {0}")]
    Unhandled(RedisError),

    #[error("Redis connection error: {0}")]
    Connection(RedisError),

    #[error("Unexpected error: {0}")]
    Unexpected(String)
}

impl From<RedisError> for Error {
    fn from(err: RedisError) -> Self {
        if err.kind() == redis::ErrorKind::IoError
            && let Some(io_err) = err.source().and_then(|e| e.downcast_ref::<io::Error>())
        {
            match io_err.kind() {
                io::ErrorKind::ConnectionRefused
                | io::ErrorKind::BrokenPipe
                | io::ErrorKind::ConnectionReset => {
                    return Error::Connection(err);
                }
                _ => {}
            }
        }
        Error::Unhandled(err)
    }
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
