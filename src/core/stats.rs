use once_cell::sync::Lazy;
pub static STATS: Lazy<Stats> = Lazy::new(Stats::new);
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::Serialize;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize)]
pub enum Counter {
    Received,
    Accepted,
    Rejected,
    Lagged,
    Ignored,
    Done,
    Failed,
    Delayed,
    Canceled,
    Waiting,
    Running
}

#[derive(Debug, Clone, Default)]
struct Tracker {
    received: Arc<AtomicUsize>,
    rejected: Arc<AtomicUsize>,
    lagged: Arc<AtomicUsize>,
    accepted: Arc<AtomicUsize>,
    ignored: Arc<AtomicUsize>,
    done: Arc<AtomicUsize>,
    failed: Arc<AtomicUsize>,
    delayed: Arc<AtomicUsize>,
    canceled: Arc<AtomicUsize>,
    waiting: Arc<AtomicUsize>,
    running: Arc<AtomicUsize>
}

impl Tracker {
    fn new() -> Self {
        Self::default()
    }

    fn increment(
        &self,
        counter: Counter
    ) {
        match counter {
            Counter::Received => self.received.fetch_add(1, Ordering::SeqCst),
            Counter::Accepted => self.accepted.fetch_add(1, Ordering::SeqCst),
            Counter::Rejected => self.rejected.fetch_add(1, Ordering::SeqCst),
            Counter::Lagged => self.lagged.fetch_add(1, Ordering::SeqCst),
            Counter::Ignored => self.ignored.fetch_add(1, Ordering::SeqCst),
            Counter::Done => self.done.fetch_add(1, Ordering::SeqCst),
            Counter::Failed => self.failed.fetch_add(1, Ordering::SeqCst),
            Counter::Delayed => self.delayed.fetch_add(1, Ordering::SeqCst),
            Counter::Canceled => self.canceled.fetch_add(1, Ordering::SeqCst),
            Counter::Waiting => self.waiting.fetch_add(1, Ordering::SeqCst),
            Counter::Running => self.running.fetch_add(1, Ordering::SeqCst)
        };
    }

    fn decrement(
        &self,
        counter: Counter
    ) {
        match counter {
            Counter::Received => self.received.fetch_sub(1, Ordering::SeqCst),
            Counter::Rejected => self.rejected.fetch_sub(1, Ordering::SeqCst),
            Counter::Lagged => self.lagged.fetch_sub(1, Ordering::SeqCst),
            Counter::Ignored => self.ignored.fetch_sub(1, Ordering::SeqCst),
            Counter::Accepted => self.accepted.fetch_sub(1, Ordering::SeqCst),
            Counter::Done => self.done.fetch_sub(1, Ordering::SeqCst),
            Counter::Failed => self.failed.fetch_sub(1, Ordering::SeqCst),
            Counter::Delayed => self.delayed.fetch_sub(1, Ordering::SeqCst),
            Counter::Canceled => self.canceled.fetch_sub(1, Ordering::SeqCst),
            Counter::Waiting => self.waiting.fetch_sub(1, Ordering::SeqCst),
            Counter::Running => self.running.fetch_sub(1, Ordering::SeqCst)
        };
    }

    fn get(
        &self,
        counter: Counter
    ) -> usize {
        match counter {
            Counter::Received => self.received.load(Ordering::SeqCst),
            Counter::Rejected => self.rejected.load(Ordering::SeqCst),
            Counter::Lagged => self.lagged.load(Ordering::SeqCst),
            Counter::Accepted => self.accepted.load(Ordering::SeqCst),
            Counter::Ignored => self.ignored.load(Ordering::SeqCst),
            Counter::Done => self.done.load(Ordering::SeqCst),
            Counter::Failed => self.failed.load(Ordering::SeqCst),
            Counter::Delayed => self.delayed.load(Ordering::SeqCst),
            Counter::Canceled => self.canceled.load(Ordering::SeqCst),
            Counter::Waiting => self.waiting.load(Ordering::SeqCst),
            Counter::Running => self.running.load(Ordering::SeqCst)
        }
    }

    fn snapshot(&self) -> Vec<(Counter, usize)> {
        vec![
            (Counter::Received, self.received.load(Ordering::SeqCst)),
            (Counter::Rejected, self.rejected.load(Ordering::SeqCst)),
            (Counter::Lagged, self.lagged.load(Ordering::SeqCst)),
            (Counter::Accepted, self.accepted.load(Ordering::SeqCst)),
            (Counter::Ignored, self.ignored.load(Ordering::SeqCst)),
            (Counter::Done, self.done.load(Ordering::SeqCst)),
            (Counter::Failed, self.failed.load(Ordering::SeqCst)),
            (Counter::Delayed, self.delayed.load(Ordering::SeqCst)),
            (Counter::Canceled, self.canceled.load(Ordering::SeqCst)),
            (Counter::Waiting, self.waiting.load(Ordering::SeqCst)),
            (Counter::Running, self.running.load(Ordering::SeqCst)),
        ]
    }
}

pub struct Stats {
    tracker: Tracker
}

#[allow(unused)]
impl Stats {
    pub fn new() -> Self {
        Stats { tracker: Tracker::new() }
    }

    pub fn increment(
        &self,
        counter: Counter
    ) {
        self.tracker.increment(counter);
    }

    pub fn decrement(
        &self,
        counter: Counter
    ) {
        self.tracker.decrement(counter);
    }

    pub fn get(
        &self,
        counter: Counter
    ) -> usize {
        self.tracker.get(counter)
    }

    pub fn snapshot(&self) -> Vec<(Counter, usize)> {
        self.tracker.snapshot()
    }

    pub fn unknown_count(&self) -> usize {
        let accepted = self.get(Counter::Accepted);
        let done = self.get(Counter::Done);
        let failed = self.get(Counter::Failed);
        let delayed = self.get(Counter::Delayed);
        let canceled = self.get(Counter::Canceled);
        accepted.saturating_sub(done + failed + delayed + canceled)
    }

    pub fn unhandled_count(&self) -> usize {
        let received = self.get(Counter::Received);
        let accepted = self.get(Counter::Accepted);
        let rejected = self.get(Counter::Rejected);
        let lagged = self.get(Counter::Lagged);
        let ignored = self.get(Counter::Ignored);
        received.saturating_sub(accepted + rejected + ignored + lagged)
    }
}

use std::fmt;

impl fmt::Display for Stats {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>
    ) -> fmt::Result {
        let parts: Vec<String> =
            self.snapshot().into_iter().map(|(c, v)| format!("{c:?}:{v}")).collect();
        write!(f, "{}", parts.join(" "))
    }
}

impl Serialize for Stats {
    fn serialize<S>(
        &self,
        serializer: S
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        use serde::ser::SerializeMap;

        let snapshot = self.snapshot();
        let mut map = serializer.serialize_map(Some(snapshot.len()))?;
        for (key, value) in snapshot {
            map.serialize_entry(&key, &value)?;
        }
        map.end()
    }
}

#[macro_export]
macro_rules! increment {
    ($counter:expr) => {
        $crate::core::stats::STATS.increment($counter)
    };
}

#[macro_export]
macro_rules! decrement {
    ($counter:expr) => {
        $crate::core::stats::STATS.decrement($counter)
    };
}

#[macro_export]
macro_rules! get {
    ($counter:expr) => {
        $crate::core::stats::STATS.get($counter)
    };
}
