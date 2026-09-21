//! Module: icp::timing
//!
//! Responsibility: measure selected typed transport boundaries across context clones.
//! Boundary: no arguments, responses, credentials or retry authority enter diagnostics.

use crate::icp::IcpCli;
use serde::Serialize;
use std::{
    cell::RefCell,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static ACTIVE: RefCell<Vec<(usize, u64)>> = const { RefCell::new(Vec::new()) };
}

/// Typed request purpose; elapsed subprocess time includes local IPC and remote waiting.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IcpRequestKind {
    Identity,
    Install,
    SetControllers,
    Start,
    Stop,
    Delete,
    Metadata,
    Query,
    Status,
    Update,
    ManagementStatus,
    AgentQuery,
}

/// Paired informational request boundary. Counts describe host requests, not internal IC hops.
#[derive(Clone, Debug, Serialize)]
pub struct IcpRequestTiming {
    pub request_id: u64,
    pub parent_request_id: Option<u64>,
    pub kind: IcpRequestKind,
    pub target: Option<String>,
    pub method: Option<String>,
    pub elapsed_micros: u128,
    /// Concurrent outer request lifetimes in this context; nested boundaries do not add workers.
    pub in_flight: u64,
    pub succeeded: Option<bool>,
}

/// Shared optional sink and request identities for one transport context.
#[derive(Default)]
pub(super) struct Timing {
    handler: Option<Box<dyn Fn(IcpRequestTiming) + Send + Sync>>,
    active: AtomicU64,
}

impl fmt::Debug for Timing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Timing").finish_non_exhaustive()
    }
}

impl IcpCli {
    /// Attach bounded diagnostic consumption before cloning this request context.
    #[must_use]
    pub fn with_timing_handler(
        mut self,
        handler: impl Fn(IcpRequestTiming) + Send + Sync + 'static,
    ) -> Self {
        self.timing = Arc::new(Timing {
            handler: Some(Box::new(handler)),
            ..Timing::default()
        });
        self
    }

    pub(crate) fn measure_request<T, E>(
        &self,
        kind: IcpRequestKind,
        target: Option<&str>,
        method: Option<&str>,
        run: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        let Some(handler) = &self.timing.handler else {
            return run();
        };
        let context = Arc::as_ptr(&self.timing) as usize;
        let parent = ACTIVE.with_borrow(|active| {
            active
                .iter()
                .rev()
                .find(|entry| entry.0 == context)
                .map(|entry| entry.1)
        });
        let in_flight = if parent.is_none() {
            self.timing.active.fetch_add(1, Ordering::SeqCst) + 1
        } else {
            self.timing.active.load(Ordering::SeqCst)
        };
        let event = IcpRequestTiming {
            request_id: NEXT_REQUEST.fetch_add(1, Ordering::Relaxed),
            parent_request_id: parent,
            kind,
            target: target.map(str::to_owned),
            method: method.map(str::to_owned),
            elapsed_micros: 0,
            in_flight,
            succeeded: None,
        };
        ACTIVE.with_borrow_mut(|active| active.push((context, event.request_id)));
        let guard = RequestGuard {
            timing: &self.timing,
            context,
            outer: parent.is_none(),
        };
        let started = Instant::now();
        handler(event.clone());
        let result = run();
        let mut event = event;
        event.elapsed_micros = started.elapsed().as_micros();
        event.succeeded = Some(result.is_ok());
        handler(event);
        drop(guard);
        result
    }
}

/// Restore diagnostic nesting on every return or panic; unmatched starts remain incomplete.
struct RequestGuard<'a> {
    timing: &'a Timing,
    context: usize,
    outer: bool,
}
impl Drop for RequestGuard<'_> {
    fn drop(&mut self) {
        ACTIVE.with_borrow_mut(|active| {
            let popped = active.pop();
            debug_assert_eq!(popped.map(|entry| entry.0), Some(self.context));
        });
        if self.outer {
            self.timing.active.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Barrier, Mutex};

    #[test]
    fn concurrent_clones_pair_requests_and_keep_nested_identity_out_of_worker_count() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        let icp = IcpCli::new("unused", None)
            .with_timing_handler(move |event| sink.lock().unwrap().push(event));
        let barrier = Barrier::new(4);
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let clone = icp.clone();
                let barrier = &barrier;
                scope.spawn(move || {
                    clone
                        .measure_request(
                            IcpRequestKind::Query,
                            Some("owner"),
                            Some("status"),
                            || {
                                barrier.wait();
                                clone.measure_request(IcpRequestKind::Identity, None, None, || {
                                    Err::<(), _>(7)
                                })
                            },
                        )
                        .unwrap_err();
                });
            }
        });
        let events = events.lock().unwrap();
        let starts = events
            .iter()
            .filter(|event| event.succeeded.is_none())
            .collect::<Vec<_>>();
        assert_eq!(
            starts
                .iter()
                .filter(|event| event.parent_request_id.is_none())
                .count(),
            4
        );
        assert_eq!(starts.iter().map(|event| event.in_flight).max(), Some(4));
        for start in starts {
            let end = events
                .iter()
                .find(|event| event.request_id == start.request_id && event.succeeded.is_some())
                .unwrap();
            assert_eq!(end.succeeded, Some(false));
            assert_eq!(end.parent_request_id, start.parent_request_id);
            if let Some(parent) = start.parent_request_id {
                assert!(
                    events
                        .iter()
                        .any(|event| event.request_id == parent
                            && event.kind == IcpRequestKind::Query)
                );
            }
        }
        drop(events);
        assert_eq!(icp.timing.active.load(Ordering::SeqCst), 0);
    }
}
