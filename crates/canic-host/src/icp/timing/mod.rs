//! Module: icp::timing
//!
//! Responsibility: measure selected typed transport boundaries across context clones.
//! Boundary: no arguments, responses, credentials or retry authority enter diagnostics.

use crate::icp::IcpCli;
use candid::Principal;
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
    static ACTIVE: RefCell<Vec<ActiveRequest>> = const { RefCell::new(Vec::new()) };
}

/// Thread-scoped attribution; inherited only by requests using the same timing context.
#[derive(Clone, Copy)]
struct ActiveRequest {
    context: usize,
    request_id: u64,
    subject: Option<Principal>,
}

/// Typed transport or inclusive protected-read boundary; durations include local and remote work.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IcpRequestKind {
    /// Inclusive local inspection, reserve query and protected status request; not another IC call.
    CanisterInspection,
    /// Inclusive protected install-history read used during reconciliation.
    CanisterHistory,
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

/// Paired informational boundary; logical reads and nested transports are not additive counts.
#[derive(Clone, Debug, Serialize)]
pub struct IcpRequestTiming {
    pub request_id: u64,
    pub parent_request_id: Option<u64>,
    pub kind: IcpRequestKind,
    pub target: Option<String>,
    /// Inspected child, distinct from the Root endpoint carrying a protected request.
    pub subject: Option<Principal>,
    pub method: Option<String>,
    pub elapsed_micros: u128,
    /// Concurrent outer operation lifetimes in this context; nested boundaries do not add workers.
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
    /// Attribute a protected history read to its child without inspecting request payloads.
    pub(crate) fn measure_canister_history<T, E>(
        &self,
        root: Principal,
        subject: Principal,
        run: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        self.measure_request_with_subject(
            IcpRequestKind::CanisterHistory,
            Some(&root.to_text()),
            None,
            Some(subject),
            run,
        )
    }

    /// Attribute one protected child inspection and its nested requests without adding effects.
    pub(crate) fn measure_canister_inspection<T, E>(
        &self,
        root: Principal,
        subject: Principal,
        run: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        self.measure_request_with_subject(
            IcpRequestKind::CanisterInspection,
            Some(&root.to_text()),
            None,
            Some(subject),
            run,
        )
    }

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
        self.measure_request_with_subject(kind, target, method, None, run)
    }

    fn measure_request_with_subject<T, E>(
        &self,
        kind: IcpRequestKind,
        target: Option<&str>,
        method: Option<&str>,
        subject: Option<Principal>,
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
                .find(|entry| entry.context == context)
                .copied()
        });
        let in_flight = if parent.is_none() {
            self.timing.active.fetch_add(1, Ordering::SeqCst) + 1
        } else {
            self.timing.active.load(Ordering::SeqCst)
        };
        let event = IcpRequestTiming {
            request_id: NEXT_REQUEST.fetch_add(1, Ordering::Relaxed),
            parent_request_id: parent.map(|parent| parent.request_id),
            kind,
            target: target.map(str::to_owned),
            subject: subject.or_else(|| parent.and_then(|parent| parent.subject)),
            method: method.map(str::to_owned),
            elapsed_micros: 0,
            in_flight,
            succeeded: None,
        };
        ACTIVE.with_borrow_mut(|active| {
            active.push(ActiveRequest {
                context,
                request_id: event.request_id,
                subject: event.subject,
            });
        });
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
            debug_assert_eq!(popped.map(|entry| entry.context), Some(self.context));
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

#[cfg(test)]
mod attribution_tests {
    use super::*;
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::{Barrier, Mutex},
    };

    #[test]
    fn concurrent_children_share_a_root_but_keep_independent_request_parents_and_subjects() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        let icp = IcpCli::new("unused", None)
            .with_timing_handler(move |event| sink.lock().unwrap().push(event));
        let root = Principal::from_slice(&[9]);
        let children = [Principal::from_slice(&[1]), Principal::from_slice(&[2])];
        let barrier = Barrier::new(children.len());
        std::thread::scope(|scope| {
            for subject in children {
                let icp = icp.clone();
                let barrier = &barrier;
                scope.spawn(move || {
                    let observed = icp.measure_canister_inspection(root, subject, || {
                        barrier.wait();
                        icp.measure_request(
                            IcpRequestKind::Query,
                            Some(&root.to_text()),
                            Some("canic_observability"),
                            || Ok::<_, u8>(()),
                        )?;
                        icp.measure_request(
                            IcpRequestKind::Update,
                            Some(&root.to_text()),
                            Some("canic_root_command"),
                            || Err::<(), _>(7),
                        )
                    });
                    assert_eq!(observed, Err(7));
                });
            }
        });
        let events = events.lock().unwrap();
        for subject in children {
            let outer = events
                .iter()
                .find(|event| {
                    event.kind == IcpRequestKind::CanisterInspection
                        && event.subject == Some(subject)
                        && event.succeeded.is_none()
                })
                .unwrap();
            assert!(outer.parent_request_id.is_none());
            let nested = events
                .iter()
                .filter(|event| event.parent_request_id == Some(outer.request_id))
                .collect::<Vec<_>>();
            assert!(nested.iter().all(|event| event.subject == Some(subject)
                && event.target.as_deref() == Some(root.to_text().as_str())));
            for kind in [IcpRequestKind::Query, IcpRequestKind::Update] {
                let start = nested
                    .iter()
                    .find(|event| event.kind == kind && event.succeeded.is_none())
                    .unwrap();
                let end = nested
                    .iter()
                    .find(|event| event.request_id == start.request_id && event.succeeded.is_some())
                    .unwrap();
                assert_eq!(end.succeeded, Some(kind == IcpRequestKind::Query));
            }
            assert!(events.iter().any(
                |event| event.request_id == outer.request_id && event.succeeded == Some(false)
            ));
        }
        drop(events);
        assert_eq!(icp.timing.active.load(Ordering::SeqCst), 0);
        assert_eq!(
            icp.remote_call_count(),
            0,
            "diagnostic scopes issue no transport calls"
        );
    }

    #[test]
    fn interrupted_and_failed_scopes_do_not_leak_subjects_or_parentage_into_later_requests() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        let icp = IcpCli::new("unused", None)
            .with_timing_handler(move |event| sink.lock().unwrap().push(event));
        let root = Principal::from_slice(&[9]);
        let subject = Principal::from_slice(&[1]);
        let other_sink = Arc::clone(&events);
        let other = IcpCli::new("unused", None)
            .with_timing_handler(move |event| other_sink.lock().unwrap().push(event));
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                icp.measure_canister_inspection(root, subject, || -> Result<(), ()> {
                    other
                        .measure_request(IcpRequestKind::Status, Some("unrelated"), None, || {
                            Ok::<_, ()>(())
                        })
                        .unwrap();
                    panic!("interrupted fixture");
                })
                .unwrap();
            }))
            .is_err()
        );
        assert_eq!(
            icp.measure_canister_history(root, subject, || Err::<(), _>(8)),
            Err(8)
        );
        icp.measure_request(IcpRequestKind::Status, Some("later"), None, || {
            Ok::<_, ()>(())
        })
        .unwrap();
        let events = events.lock().unwrap();
        let interrupted = events
            .iter()
            .find(|event| event.kind == IcpRequestKind::CanisterInspection)
            .unwrap();
        assert!(
            events
                .iter()
                .filter(|event| event.request_id == interrupted.request_id)
                .all(|event| event.succeeded.is_none())
        );
        assert!(
            events
                .iter()
                .filter(|event| event.kind == IcpRequestKind::Status)
                .all(|event| event.subject.is_none() && event.parent_request_id.is_none())
        );
        let history = events
            .iter()
            .find(|event| {
                event.kind == IcpRequestKind::CanisterHistory && event.succeeded == Some(false)
            })
            .unwrap();
        assert_eq!(history.subject, Some(subject));
        assert!(history.parent_request_id.is_none());
        drop(events);
        assert_eq!(icp.timing.active.load(Ordering::SeqCst), 0);
    }
}
