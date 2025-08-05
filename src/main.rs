mod actions;
mod core;
mod ctx;
mod error;
mod svc;

pub use error::Result;
use tokio::join;

use self::core::shutdown;
use crate::ctx::{State, logging};
use crate::svc::{dispatcher, pubsub};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result {
    dotenvy::dotenv().ok();

    logging::init_log().await?;

    let state = State::shared()?;

    tokio::spawn(shutdown::listen(state.clone()));

    log::debug!("Options: {:?}", state.options);

    let subscriber = tokio::spawn(pubsub::start_subscriber(state.clone()));
    let dispatcher = tokio::spawn(dispatcher::run(state.clone()));

    let result = join!(subscriber, dispatcher);

    let (subscriber_result, dispatcher_result) = result;

    if let Err(e) = subscriber_result {
        log::error!("Subscriber task failed: {:?}", e);
    }

    if let Err(e) = dispatcher_result {
        log::error!("Dispatcher task failed: {:?}", e);
    }

    Ok(())
}
