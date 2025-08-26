use crate::ctx::CtxError;
use crate::ctx::logging::Error as LoggingError;

pub type Result = std::result::Result<(), Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Ctx Failed: {0}")]
    Ctx(#[from] CtxError),

    #[error("Log error: {0}")]
    Logging(#[from] LoggingError),

    #[error("Dispatcher failed: {0}")]
    Dispatcher(#[from] crate::svc::dispatcher::Error),

    #[error("Subscriber failed: {0}")]
    Subscriber(#[from] crate::svc::pubsub::Error)
}
