use broadcast::{Receiver, Sender, channel};
use tokio::sync::broadcast;

use crate::core::Command;
use crate::core::error::Error;
use crate::core::stats::Counter;
use crate::{State, increment};

pub struct BroadcastManager {
    sender: Sender<Command>
}

impl BroadcastManager {
    pub fn new() -> Self {
        let (sender, _) = channel(100);
        Self { sender }
    }

    pub fn subscribe(&self) -> Receiver<Command> {
        self.sender.subscribe()
    }

    pub fn close(&self) {
        // Dropping the sender will close the channel
        drop(self.sender.clone());
    }

    //
    pub fn sender(&self) -> Sender<Command> {
        self.sender.clone()
    }
}

impl Default for BroadcastManager {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn send_command(
        &self,
        command: Command
    ) -> Result<(), Error> {
        if !self.is_shutting_down() {
            let _ = self
                .broadcast
                .sender()
                .send(command.clone())
                .map_err(|_| Error::Internal("Error sending command".to_string()));
        } else {
            increment!(Counter::Rejected);
            log::warn!("⛔ Cannot send command, shutdown is in progress");
        }
        Ok(())
    }
}
