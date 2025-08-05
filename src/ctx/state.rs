use std::sync::Arc;

use clap::Parser;
use tokio_util::sync::CancellationToken;

use super::error::Error;
use super::{Info, Options};
use crate::core::BroadcastManager;

pub type SharedState = Arc<State>;

#[allow(unused)]
pub struct State {
    pub options: Options,
    pub info: Info,
    shutdown_token: CancellationToken,
    pub broadcast: BroadcastManager
}

impl State {
    pub fn shared() -> Result<Arc<Self>, Error> {
        let options = Options::parse();
        Ok(Arc::new(Self {
            options,
            info: Info::from_env()?,
            broadcast: BroadcastManager::default(),
            shutdown_token: CancellationToken::new()
        }))
    }
}

#[allow(unused)]
impl State {
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_token.is_cancelled()
    }

    pub fn initiate_shutdown(&self) {
        self.shutdown_token.cancel();
    }

    pub fn on_shutdown(&self) -> impl Future<Output = ()> + '_ {
        self.shutdown_token.cancelled()
    }

    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown_token.clone()
    }
}
