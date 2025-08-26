use std::sync::atomic::{AtomicU8, Ordering};

use futures_util::StreamExt;
use once_cell::sync::Lazy;
use tokio::time::Duration;

use super::error::Error;
use crate::core::stats::Counter;
use crate::ctx::SharedState;
use crate::increment;
use crate::svc::pubsub::messages::handle_message;

static RETRY_COUNTER: Lazy<AtomicU8> = Lazy::new(|| AtomicU8::new(0));

pub async fn start_subscriber(state: SharedState) -> crate::Result {
    const SHORT_RETRY_COUNT: u8 = 150;
    const SHORT_DELAY_SECONDS: u64 = 2;
    const LONG_DELAY_SECONDS: u64 = 60;

    loop {
        log::debug!("Starting subscription service");

        if state.is_shutting_down() {
            log::warn!("Shutdown detected, re-subscription canceled");
            break;
        }

        match subscribe_channel(state.clone()).await {
            Ok(_) => {
                log::debug!("❎ Subscription ended gracefully.");
                break;
            }
            Err(e) => match &e {
                Error::RedisDisconnected => {
                    log::error!("Redis disconnected: {}", e);
                    let count = RETRY_COUNTER.fetch_add(1, Ordering::SeqCst);
                    let delay = if count < SHORT_RETRY_COUNT {
                        SHORT_DELAY_SECONDS
                    } else {
                        LONG_DELAY_SECONDS
                    };
                    log::warn!("Restarting subscriber in {} seconds...,", delay);
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                }
                Error::RedisConnectionError(conn_err) => {
                    log::error!("Redis connection failed: {}", conn_err);
                    let count = RETRY_COUNTER.fetch_add(1, Ordering::SeqCst);
                    let delay = if count < SHORT_RETRY_COUNT {
                        SHORT_DELAY_SECONDS
                    } else {
                        LONG_DELAY_SECONDS
                    };
                    log::warn!("Restarting subscriber in {} seconds...,", delay);
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                }
                Error::UnhandledRedisError(redis_err) => {
                    log::error!("Unhandled Redis error: {} closing subscriber", redis_err);
                    log::debug!("Full redis error trace: {:?}", e);
                    return Err(e.into());
                }
                _ => {
                    log::error!("Unexpected error: {}", e);
                    log::debug!("Full error trace: {:?}", e);
                    return Err(e.into());
                }
            }
        }
    }
    Ok(())
}

async fn subscribe_channel(state: SharedState) -> Result<(), Error> {
    let options = &state.options;

    let client = redis::Client::open(options.redis_url.as_str())?;
    let mut subscriber = client.get_async_pubsub().await?;

    subscriber.subscribe(&options.channel).await?;
    RETRY_COUNTER.store(0, Ordering::SeqCst);
    log::info!("Subscribed to channel '{}'", &options.channel);

    let graceful_timeout = options.grace_timeout.unwrap_or(Duration::from_secs(1));

    let result = async {
        let mut msg_stream = subscriber.on_message();
        loop {
            tokio::select! {
                _ = state.on_shutdown() => {
                    log::warn!("🔻 Subscriber is shutting down");
                    break;
                }

                result = msg_stream.next() => {

                    match result {
                        Some(msg) => {
                            let handle_result = tokio::time::timeout(
                                graceful_timeout,
                                handle_message(state.clone(), msg),
                            ).await;

                            match handle_result {
                                Ok(Ok(_)) => {},
                                Ok(Err(e)) => {
                                    increment!(Counter::Rejected);
                                    log::error!("Error handling message: {e:?}");
                                },
                                Err(_) => {
                                    increment!(Counter::Rejected);
                                    log::error!("Message handling timed out after {:?}", graceful_timeout);
                                },
                            }
                        }
                        None => {
                            return Err(Error::RedisDisconnected);
                        }
                    }
                }
            }
        }
        Ok(())
    }
    .await;

    match result {
        Ok(_) => {
            if let Err(e) = subscriber.unsubscribe(&options.channel).await {
                log::warn!("❌ Unsubscribe failed during graceful shutdown: {}", e);
            }
            log::info!("📴 Unsubscribed from channel '{}'", &options.channel);
            Ok(())
        }
        Err(e @ Error::RedisConnectionError(_) | e @ Error::RedisDisconnected) => Err(e),
        Err(e) => {
            if let Err(e) = subscriber.unsubscribe(&options.channel).await {
                log::warn!("Unsubscribe failed during graceful shutdown: {}", e);
            }
            log::error!("❌ Subscription loop exited with error: {}", e);
            Err(e)
        }
    }
}
