use std::time::Duration;

use rand::Rng;
use tokio::time;
use tokio_util::sync::CancellationToken;

use crate::core::handle::{Error as HandleError, Handle, Watcher};
use crate::core::stats::{Counter, STATS};
use crate::ctx::SharedState;
use crate::{decrement, increment};

enum TaskResult {
    Success,
    Canceled,
    Delayed,
    Failed(TaskError)
}

#[derive(Debug)]
pub enum TaskError {
    Unimplemented
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unhandled commands: {0}")]
    UnhandledCommands(usize),

    #[error("unknown tasks: {0}")]
    UnknownTasks(usize)
}

pub async fn run(state: SharedState) -> crate::Result {
    let handle =
        create_handle(state.shutdown_token(), state.options.workers, state.options.grace_timeout);
    let mut receiver_tx = state.broadcast.subscribe();
    let mut task_id: u32 = 0;

    loop {
        tokio::select! {
            _ = state.on_shutdown() => {
                log::warn!("🔸 Dispatcher got shutdown signal ");
                log::debug!("📉 Unhandled count at shutdown: {}", STATS.unhandled_count());

                loop {
                    let unhandled = STATS.unhandled_count();

                    if unhandled == 0 {
                        break;
                    } else {
                        log::trace!("🔍 Unhandled commands: {}", unhandled);
                    }

                    match receiver_tx.recv().await {
                        Ok(command) => {
                            increment!(Counter::Rejected);
                            log::trace!("🔥 Command `{:?}` rejected during shutdown.", command);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            log::error!("📴 Channel closed, no more commands to process.");
                            break;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            increment!(Counter::Lagged);
                            log::error!("⚠️  ‼️  Broadcast lagged, skipping termination");
                        }
                    }
                }

                log::warn!("🔻 Dispatcher is shutting down.");
                break;
            }

            result = receiver_tx.recv() => match result {
                Ok(command) => {
                    log::debug!("📩 Received command: {:?}", command);

                    increment!(Counter::Waiting);

                    let watcher = match handle.try_acquire_watcher().await {
                        Ok(w) => {
                            decrement!(Counter::Waiting);
                            increment!(Counter::Accepted);
                            w
                        }
                        Err(HandleError::ShuttingDown) => {
                            decrement!(Counter::Waiting);
                            increment!(Counter::Rejected);
                            log::debug!("🔥 Shutdown initiated — job is not permitted");
                            continue;
                        }
                    };

                    task_id += 1;

                    log::debug!("🔹 Task #{} acquired permit. {} running ", task_id, handle.count());

                    let state = state.clone();
                    let task = tokio::spawn(async move {
                        increment!(Counter::Running);
                        run_job(task_id, state, watcher).await
                    });

                    let started_at = time::Instant::now();
                    // let handle_clone = handle.clone();
                    tokio::spawn(async move {
                        let task_result = match task.await {
                            Ok(inner) => inner,
                            Err(err) => {
                                decrement!(Counter::Running);
                                increment!(Counter::Failed);
                                log::error!("⚠️  ‼️  Task spawn error for Task#{task_id} elapsed time: {err}");
                                return;
                            }
                        };
                        decrement!(Counter::Running);
                        let job_result = task_result;
                        let elapsed = started_at.elapsed();
                        match job_result {
                            TaskResult::Success => {
                                STATS.increment(Counter::Done);
                                log::info!("❎ Task #{task_id} successfully done, elapsed: {:.2?}", elapsed);
                            }
                            TaskResult::Delayed => {
                                STATS.increment(Counter::Delayed);
                                log::warn!("🟡 Task #{task_id} pushed to queue runner: elapsed: {:.2?}", elapsed);
                            }
                            TaskResult::Canceled => {
                                STATS.increment(Counter::Canceled);
                                log::error!(
                                    "📛 Task #{task_id} canceled due to shutdown forced, elapsed: {:.2?}",
                                    elapsed
                                );
                            }
                            TaskResult::Failed(err) => {
                                STATS.increment(Counter::Failed);
                                log::error!("❌ Task #{task_id} failed, elapsed: {:.2?} {err:?}", elapsed);
                            }
                        }
                        // log::info!("Waiting tasks {}",handle_clone.count());
                    });
                }
                Err(err) => match err {
                    tokio::sync::broadcast::error::RecvError::Closed => {
                        log::warn!("📴 Channel closed, no more commands to process.");
                        handle.graceful_shutdown(state.options.grace_timeout);
                        break
                    }
                    tokio::sync::broadcast::error::RecvError::Lagged(_) => {
                        increment!(Counter::Lagged);
                        log::error!("⚠️  ‼️  Broadcast lagged, skipping command");
                        handle.graceful_shutdown(state.options.grace_timeout);
                        break
                    }
                },
            }
        }
    }

    log::warn!(
        "🧭 Dispatcher Waiting for {} connections to finish with max duration {:?}",
        handle.count(),
        handle.grace_period()
    );

    handle.wait_all_done().await;

    tokio::time::sleep(Duration::from_millis(1)).await; // Wait for canceled job results. because we work in instantaneous, we must wait cancellation task result.

    log::info!("📊 Final stats: {}", *STATS);

    let loss_count = STATS.unknown_count();
    if loss_count > 0 {
        return Err(Error::UnknownTasks(loss_count).into());
    }

    let unhandled_count = STATS.unhandled_count();
    if unhandled_count > 0 {
        return Err(Error::UnhandledCommands(unhandled_count).into());
    }

    Ok(())
}

pub fn create_handle(
    token: CancellationToken,
    max_count: Option<usize>,
    grace_timeout: Option<Duration>
) -> Handle {
    let handle = Handle::new(max_count);
    let cloned_handle = handle.clone();
    tokio::spawn(async move {
        // Wait for the cancellation token to be triggered
        token.cancelled().await;
        // Log the shutdown message
        log::debug!("💥 Handle notified for graceful shutdown...");
        // Perform graceful shutdown with the specified grace timeout
        cloned_handle.graceful_shutdown(grace_timeout);
    });
    handle
}

async fn run_job(
    job_id: u32,
    state: SharedState,
    watcher: Watcher
) -> TaskResult {
    log::debug!("▶️  Task #{} started...", job_id);
    let max_random_from_idle_timeout = state
        .options
        .idle_timeout
        .unwrap_or(Duration::from_secs(5))
        .as_millis()
        .min(u128::from(u32::MAX)) as u64;
    let random_ms = rand::rng().random_range(1..=max_random_from_idle_timeout);
    tokio::select! {
        _ = watcher.wait_graceful_shutdown() => {
           log::debug!("🫡 Task #{} notified for shutdown...", job_id);
             let max_random_from_grace_timeout =  2 *  state.options.grace_timeout.unwrap_or(Duration::from_secs(1)).as_millis().min(u128::from(u32::MAX)) as u64;
             let random_ms = rand::rng().random_range(1..=max_random_from_grace_timeout);
            tokio::select! {
                _ = watcher.wait_shutdown() => {TaskResult::Canceled}
                _ = time::sleep(Duration::from_millis(random_ms)) => {TaskResult::Delayed}
            }
        }
        _ = watcher.wait_shutdown() => {
             TaskResult::Canceled
        }
        _ = time::sleep(Duration::from_millis(random_ms)) => {
            if random_ms % 5 == 0 {
                TaskResult::Failed(TaskError::Unimplemented)
            } else {
                TaskResult::Success
            }
        }
    }
}
