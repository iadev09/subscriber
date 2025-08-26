use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time::sleep;

use crate::core::notify::NotifyOnce;

#[derive(Clone, Debug, Default)]
pub struct Handle {
    inner: Arc<Inner>
}

#[derive(Debug, Default)]
struct Inner {
    graceful: NotifyOnce,
    shutdown: NotifyOnce,
    released: Notify,
    count: AtomicUsize,
    all_done: NotifyOnce,
    grace_period: Mutex<Option<Duration>>,
    max_count: Option<usize>
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Service shutting down")]
    ShuttingDown // GracefulShutdown(Duration),
}

#[allow(unused)]
impl Handle {
    /// Create a new handle.
    pub fn new(max_count: Option<usize>) -> Self {
        if let Some(max_count) = max_count {
            let mut inner = Inner::default();
            inner.max_count = Some(max_count);

            Handle { inner: Arc::new(inner) }
        } else {
            Handle::default()
        }
    }

    pub(crate) fn watcher(&self) -> Watcher {
        Watcher::new(self.clone())
    }

    /// Returns the current grace period duration (if any).
    pub fn grace_period(&self) -> Option<Duration> {
        *self.inner.grace_period.lock().unwrap()
    }

    /// Get the number of connections.
    pub fn count(&self) -> usize {
        self.inner.count.load(Ordering::SeqCst)
    }

    /// Shutdown the server.
    pub(self) fn shutdown(&self) {
        self.inner.shutdown.notify_waiters();
    }

    /// Gracefully shutdown the server.
    ///
    /// `None` means indefinite grace period.
    pub fn graceful_shutdown(
        &self,
        duration: Option<Duration>
    ) {
        *self.inner.grace_period.lock().unwrap() = duration;

        self.inner.graceful.notify_waiters();
    }

    pub(self) fn is_shutting_down(&self) -> bool {
        self.inner.shutdown.is_notified()
    }

    pub(self) async fn wait_shutdown(&self) {
        self.inner.shutdown.notified().await;
    }

    pub async fn wait_graceful_shutdown(&self) {
        self.inner.graceful.notified().await;
    }

    pub(crate) async fn try_acquire_watcher(&self) -> Result<Watcher, Error> {
        loop {
            if self.inner.graceful.is_notified() {
                return Err(Error::ShuttingDown);
            }

            let count = self.inner.count.load(Ordering::SeqCst);

            if let Some(max_count) = self.inner.max_count {
                if count < max_count {
                    return Ok(self.watcher());
                }
            }

            // Wait until a connection is freed
            self.inner.released.notified().await;
            // Loop again to check if you can enter
        }
    }

    pub(crate) async fn wait_all_done(&self) {
        if self.inner.count.load(Ordering::SeqCst) == 0 {
            return;
        }

        let deadline = *self.inner.grace_period.lock().unwrap();

        match deadline {
            Some(duration) => tokio::select! {
                biased;
                _ = sleep(duration) => self.shutdown(),
                _ = self.inner.all_done.notified() => (),
            },
            None => self.inner.all_done.notified().await
        }
    }
}

pub(crate) struct Watcher {
    handle: Handle
}

#[allow(unused)]
impl Watcher {
    fn new(handle: Handle) -> Self {
        handle.inner.count.fetch_add(1, Ordering::SeqCst);

        Self { handle }
    }

    pub(crate) async fn wait_graceful_shutdown(&self) {
        self.handle.wait_graceful_shutdown().await
    }

    pub(crate) async fn wait_shutdown(&self) {
        self.handle.wait_shutdown().await
    }

    pub(crate) fn is_shutting_down(&self) -> bool {
        self.handle.is_shutting_down()
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        let count = self.handle.inner.count.fetch_sub(1, Ordering::SeqCst) - 1;

        if count == 0 && self.handle.inner.graceful.is_notified() {
            self.handle.inner.all_done.notify_waiters();
        }

        // watcher not dropped yet.
        if let Some(max_count) = self.handle.inner.max_count {
            if count < max_count {
                // Notify waiters that a slot is available
                self.handle.inner.released.notify_waiters();
            }
        }
    }
}
