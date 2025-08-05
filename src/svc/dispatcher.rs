use std::time::Duration;

use rand::Rng;
use tokio::time;
use tokio_util::sync::CancellationToken;

use crate::core::AppError;
use crate::core::shutdown::handle::{Handle, HandleError, Watcher};
use crate::core::stats::{Counter, STATS};
use crate::ctx::SharedState;
use crate::{Result, decrement, increment};

pub async fn run(state: SharedState) -> Result {
    let handle =
        create_handle(state.shutdown_token(), state.options.workers, state.options.grace_timeout);

    let mut receiver_tx = state.broadcast.subscribe();

    let mut job_id: u32 = 0;

    loop {
        tokio::select! {

            _ = state.on_shutdown() => {
                log::warn!("🔥 Dispatcher Shutting down");
                break;
            }

            Ok(command) =  receiver_tx.recv() => {
                log::debug!("Received command: {:?}", command);

                increment!(Counter::Waiting);

                let watcher = match handle.try_acquire_watcher().await {
                    Ok(w) => w,
                    Err(HandleError::Rejected) => {
                        decrement!(Counter::Waiting);
                        increment!(Counter::Rejected);
                        log::warn!("🛑 Shutdown initiated — no more jobs will be permitted.");
                        break;
                    }
                };

                decrement!(Counter::Waiting);
                increment!(Counter::Accepted);

                job_id += 1;

                log::debug!("🔹 Job acquired permit. {} running ", handle.count());
                let state = state.clone();
                let task = tokio::spawn(async move {
                    increment!(Counter::Running);
                    run_job(job_id, state, watcher).await
                });

                let started_at = time::Instant::now();
                tokio::spawn(async move {
                    let task_result = match task.await {
                        Ok(inner) => inner,
                        Err(err) => {
                            decrement!(Counter::Running);
                            increment!(Counter::Failed);
                            log::error!("‼️⚠️   Task spawn error for Job#{job_id} elapsed time: {err}");
                            return;
                        }
                    };
                    decrement!(Counter::Running);
                    let job_result = task_result;
                    let elapsed = started_at.elapsed();
                    match job_result {
                        JobResult::Success => {
                            STATS.increment(Counter::Done);
                            log::info!("❎ Job#{job_id} successfully done, elapsed: {:.2?}", elapsed);
                        }
                        JobResult::Delayed => {
                            STATS.increment(Counter::Delayed);
                            log::warn!("🟡 Job#{job_id} pushed to queue runner: elapsed: {:.2?}:", elapsed);
                        }
                        JobResult::Canceled => {
                            STATS.increment(Counter::Canceled);
                            log::warn!(
                                "📛 Job#{job_id} canceled due to shutdown forced, elapsed: {:.2?}",
                                elapsed
                            );
                        }
                        JobResult::Failed(err) => {
                            STATS.increment(Counter::Failed);
                            log::error!("❌ Job#{job_id} failed, elapsed: {:.2?} {err}", elapsed);
                        }
                    }
                });

            }
        }
    }

    log::info!(
        "🧭 HTTP/2 Server Waiting for {} connections to finish with max duration {:?}",
        handle.count(),
        handle.grace_period()
    );

    handle.wait_all_done().await;

    log::info!("❎ Server shutdown successfully");

    log::info!("📉 Final stats: {}", STATS.to_string());

    let loss_count = STATS.loss_count();
    log::info!("📉 Loss count: {}", loss_count);

    // time::sleep(Duration::from_millis(500)).await;

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
) -> JobResult {
    log::info!("▶️  Job #{} started...", job_id);
    let random_ms = rand::rng().random_range(1..=10000);
    tokio::select! {
        _ = watcher.wait_graceful_shutdown() => {
           log::warn!("🫡 Job #{} notified for shutdown...", job_id);
             let max_random_from_grace_timeout = state.options.grace_timeout.unwrap_or(Duration::from_secs(5)).as_millis().min(u128::from(u32::MAX)) as u64;
             let random_ms = rand::rng().random_range(1..=max_random_from_grace_timeout);
            tokio::select! {
                _ = watcher.wait_shutdown() => {JobResult::Canceled}
                _ = time::sleep(Duration::from_millis(random_ms)) => {JobResult::Delayed}
            }
        }
        _ = watcher.wait_shutdown() => {
             JobResult::Canceled
        }
        _ = time::sleep(Duration::from_millis(random_ms)) => {
            if random_ms % 5 == 0 {
                JobResult::Failed(AppError::Unimplemented)
            } else {
                JobResult::Success
            }
        }
    }
}

enum JobResult {
    Success,
    Canceled,
    Delayed,
    Failed(AppError)
}
