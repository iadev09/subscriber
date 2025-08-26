mod core;
mod ctx;
mod error;
mod svc;

pub use error::{Error as AppError, Result};

use crate::ctx::{State, logging};
use crate::svc::{dispatcher, pubsub, shutdown};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result {
    dotenvy::dotenv().ok();

    logging::init_log().await?;

    let state = State::shared()?;

    tokio::spawn(shutdown::listen(state.clone()));

    log::debug!("Options: {:?}", state.options);

    let subscriber = pubsub::run(state.clone());

    let dispatcher = dispatcher::run(state.clone());

    match tokio::try_join!(subscriber, dispatcher) {
        Ok((_, _)) => {
            log::info!("❎ Subscriber and Dispatcher completed successfully.");
        }
        Err(err) => {
            log::error!("❌ Service failed: {err}");
            return Err(err);
        }
    }

    log::info!("✅ {} exits successfully! 🎉", state.info.app);

    Ok(())
}
