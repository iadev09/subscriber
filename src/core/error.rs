use tokio::task::JoinError;

use crate::core::shutdown::handle::HandleError;
use crate::ctx::CtxError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Ctx Failed: {0}")]
    Ctx(#[from] CtxError),

    #[error("Shutdown: {0}")]
    Handle(#[from] HandleError),

    #[error("Task join error: {0}")]
    JoinError(#[from] JoinError),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Unimplemented")]
    Unimplemented
}

impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Error::Internal(error.to_string())
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Error::Internal(error)
    }
}
