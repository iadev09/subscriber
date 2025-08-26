use tokio::signal::unix::{SignalKind, signal};

use crate::ctx::SharedState;

pub async fn listen(state: SharedState) {
    let mut terminate_signal =
        signal(SignalKind::terminate()).expect("Failed to create terminate signal handler");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                log::debug!("🔥 Ctrl-C received, initiating shutdown");
                state.initiate_shutdown();
            }
            _ = terminate_signal.recv() => {
                log::debug!("🔥 Terminate signal received, initiating shutdown");
                state.initiate_shutdown();
            }
            // _ = state.on_shutdown() => {
            //     log::info!("❎ Shutdown completed");
            //     // return;
            // }
        }
    }
}
