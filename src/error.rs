use tokio::task::JoinError;

use crate::ctx::CtxError;
use crate::ctx::logging::Error as LoggingError;

pub type Result = std::result::Result<(), Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Ctx Failed: {0}")]
    Ctx(#[from] CtxError),

    #[error("Log error: {0}")]
    Logging(#[from] LoggingError),

    #[error("Task join error: {0}")]
    JoinError(#[from] JoinError)
}
