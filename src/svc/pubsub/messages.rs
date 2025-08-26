use redis::Msg;
use serde_json::Value;

use super::error::Error;
use crate::core::Command;
use crate::core::stats::Counter;
use crate::ctx::SharedState;
use crate::increment;

pub async fn handle_message(
    state: SharedState,
    msg: Msg
) -> Result<(), Error> {
    increment!(Counter::Received);

    let payload: String = msg.get_payload()?;

    // Parse the payload as JSON
    let json: Value = match serde_json::from_str(&payload) {
        Ok(v) => {
            let compact = serde_json::to_string(&v)?;
            log::trace!("Received message on channel {}: {}", msg.get_channel_name(), compact);
            v
        }
        Err(e) => {
            log::warn!("Received invalid JSON: {payload}. Error: {e}");
            return Ok(()); // Return Ok to continue processing messages
        }
    };

    // Extract the event name
    let event_name = json["event"].as_str().unwrap_or("unknown");

    log::debug!("📥 Received message: {}", event_name);

    match event_name {
        "env.updated" => {
            if let Some(_d) = json.get("data") {
                let _ = state.send_command(Command::Run);
            } else {
                log::error!("❓Received version.updated event without data");
                increment!(Counter::Rejected);
            }
        }
        "env.shutdown" => {
            if let Some(data) = json.get("data") {
                if let Some(services) = data.get("services").and_then(|a| a.as_array()) {
                    let my_name = state.info.my_name();
                    if services
                        .iter()
                        .any(|v| v.as_str() == Some("*") || v.as_str() == Some(my_name))
                    {
                        // state.send_command(Command::Shutdown)?;
                        log::warn!("🔸 Received shutdown message targeting: {}", my_name);
                        increment!(Counter::Accepted);
                        increment!(Counter::Done);
                        state.initiate_shutdown();
                    } else {
                        log::debug!("⚠️  Shutdown message ignored, not targeting: {}", my_name);
                        increment!(Counter::Ignored);
                    }
                } else {
                    log::error!("⚠️  Received version.shutdown event without services");
                    increment!(Counter::Rejected);
                }
            } else {
                log::error!("⚠️  Received version.shutdown event without data");
                increment!(Counter::Rejected);
            }
        }
        _ => {
            log::debug!("Received message with unknown event: {event_name}");
            increment!(Counter::Ignored);
        }
    }

    Ok(())
}
