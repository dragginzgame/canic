//! Module: pic::timing
//!
//! Responsibility: retain nested monotonic phase evidence for governed journeys.
//! Boundary: diagnostic records do not change test outcomes or runtime pacing.

#[cfg(test)]
mod tests;

use std::{
    cell::RefCell,
    io::{self, Write},
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use canic_host::fleet_ensure::dto::FleetObservationTiming;
use serde::Serialize;

thread_local! {
    static ACTIVE: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// A phase remains incomplete until its owner explicitly finishes it.
pub(super) struct Span {
    id: u64,
    parent_id: Option<u64>,
    name: &'static str,
    started: Instant,
    finished: bool,
    // Parentage belongs to this thread; moving a live span would corrupt it.
    thread: PhantomData<Rc<()>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum State {
    Started,
    Completed,
    Failed,
    Incomplete,
}

#[derive(Debug, Serialize)]
struct TimingEvent {
    schema_version: u8,
    process_id: u32,
    span_id: u64,
    parent_id: Option<u64>,
    name: &'static str,
    state: State,
    elapsed_us: u128,
}

#[derive(Debug, Serialize)]
struct ObservationEvent {
    schema_version: u8,
    process_id: u32,
    parent_id: Option<u64>,
    observation: FleetObservationTiming,
}

impl ObservationEvent {
    fn new(observation: FleetObservationTiming) -> Self {
        Self {
            schema_version: 1,
            process_id: std::process::id(),
            parent_id: ACTIVE.with_borrow(|active| active.last().copied()),
            observation,
        }
    }
}

pub(super) fn observation(timing: FleetObservationTiming) {
    emit("CANIC-OBSERVATION", &ObservationEvent::new(timing));
}

impl Span {
    pub(super) fn start(name: &'static str) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let parent_id = ACTIVE.with_borrow_mut(|active| {
            let parent = active.last().copied();
            active.push(id);
            parent
        });
        let span = Self {
            id,
            parent_id,
            name,
            started: Instant::now(),
            finished: false,
            thread: PhantomData,
        };
        emit("CANIC-TIMING", &span.record(State::Started));
        span
    }

    pub(super) fn finish(mut self) {
        self.close(State::Completed);
    }

    pub(super) fn next(self, name: &'static str) -> Self {
        self.finish();
        Self::start(name)
    }

    fn record(&self, state: State) -> TimingEvent {
        TimingEvent {
            schema_version: 1,
            process_id: std::process::id(),
            span_id: self.id,
            parent_id: self.parent_id,
            name: self.name,
            state,
            elapsed_us: self.started.elapsed().as_micros(),
        }
    }

    fn close(&mut self, state: State) {
        if self.finished {
            return;
        }
        self.finished = true;
        ACTIVE.with_borrow_mut(|active| active.retain(|id| *id != self.id));
        emit("CANIC-TIMING", &self.record(state));
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        self.close(if std::thread::panicking() {
            State::Failed
        } else {
            State::Incomplete
        });
    }
}

fn emit(prefix: &str, event: &impl Serialize) {
    if let Ok(json) = serde_json::to_string(event) {
        // Emit one buffer so libtest stdout cannot split a JSON event between fields.
        let line = format!("[{prefix}] {json}\n");
        let mut stderr = io::stderr().lock();
        let _ = stderr.write_all(line.as_bytes());
        let _ = stderr.flush();
    }
}
