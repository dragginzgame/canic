//! Async diagnostic pairing and cancellation without thread-local attribution leaks.

use super::*;
use std::{
    future::{pending, ready},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Mutex,
    task::{Context, Poll, Waker},
};

fn context() -> (IcpCli, Arc<Mutex<Vec<IcpRequestTiming>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    let icp = IcpCli::new("unused", None)
        .with_timing_handler(move |event| sink.lock().unwrap().push(event));
    (icp, events)
}

#[test]
fn interleaved_async_requests_keep_subjects_and_results_independent() {
    let (icp, events) = context();
    let clone = icp.clone();
    let root = Principal::from_slice(&[9]);
    let first_child = Principal::from_slice(&[1]);
    let second_child = Principal::from_slice(&[2]);
    let mut cx = Context::from_waker(Waker::noop());
    let mut first = Box::pin(icp.measure_async_request(
        IcpRequestKind::AgentUpdate,
        root,
        "canic_root_command",
        Some(first_child),
        pending::<Result<(), u8>>(),
    ));
    assert!(first.as_mut().poll(&mut cx).is_pending());
    let mut second = Box::pin(clone.measure_async_request(
        IcpRequestKind::AgentQuery,
        root,
        "canic_root_status",
        Some(second_child),
        ready(Err::<(), _>(7_u8)),
    ));
    assert_eq!(second.as_mut().poll(&mut cx), Poll::Ready(Err(7)));
    let events = events.lock().unwrap();
    let starts = events
        .iter()
        .filter(|event| event.succeeded.is_none())
        .collect::<Vec<_>>();
    assert_eq!(starts.len(), 2);
    assert_ne!(starts[0].request_id, starts[1].request_id);
    assert_eq!(starts[0].subject, Some(first_child));
    assert_eq!(starts[1].subject, Some(second_child));
    assert_eq!(starts[1].in_flight, 2);
    assert!(events.iter().all(|event| event.parent_request_id.is_none()));
    let end = events
        .iter()
        .find(|event| event.succeeded.is_some())
        .unwrap();
    assert_eq!(end.request_id, starts[1].request_id);
    assert_eq!(end.subject, starts[1].subject);
    assert_eq!(end.target, starts[1].target);
    assert_eq!(end.method, starts[1].method);
    assert_eq!(end.succeeded, Some(false));
    drop(events);
    drop(first);
    assert_eq!(icp.timing.active.load(Ordering::SeqCst), 0);
    assert_eq!(icp.remote_call_count(), 0);
}

#[test]
fn cancelled_async_request_is_incomplete_without_contaminating_later_scopes() {
    let (icp, events) = context();
    let child = Principal::from_slice(&[1]);
    let mut cx = Context::from_waker(Waker::noop());
    let mut request = Box::pin(icp.measure_async_request(
        IcpRequestKind::AgentRequestStatus,
        child,
        "read_state",
        Some(child),
        pending::<Result<(), u8>>(),
    ));
    // Constructing an unpolled request is not an observed transport start.
    assert!(events.lock().unwrap().is_empty());
    assert!(request.as_mut().poll(&mut cx).is_pending());
    icp.measure_request(IcpRequestKind::Identity, None, None, || Ok::<_, u8>(()))
        .unwrap();
    drop(request);
    assert_eq!(icp.timing.active.load(Ordering::SeqCst), 0);
    let mut later = Box::pin(icp.measure_async_request(
        IcpRequestKind::AgentQuery,
        child,
        "later",
        None,
        ready(Ok::<_, u8>(5)),
    ));
    assert_eq!(later.as_mut().poll(&mut cx), Poll::Ready(Ok(5)));
    let events = events.lock().unwrap();
    let cancelled = &events[0];
    assert_eq!(
        events
            .iter()
            .filter(|event| event.request_id == cancelled.request_id)
            .count(),
        1
    );
    assert!(
        events
            .iter()
            .filter(|event| event.request_id != cancelled.request_id)
            .all(|event| event.subject.is_none() && event.parent_request_id.is_none())
    );
    assert!(
        events
            .iter()
            .filter(|event| event.method.as_deref() == Some("later"))
            .all(|event| event.in_flight == 1)
    );
    drop(events);
    assert_eq!(icp.timing.active.load(Ordering::SeqCst), 0);
}

#[test]
fn async_panics_release_counts_and_preserve_unmatched_start() {
    let (icp, events) = context();
    let mut cx = Context::from_waker(Waker::noop());
    let mut request = Box::pin(icp.measure_async_request(
        IcpRequestKind::AgentUpdate,
        Principal::management_canister(),
        "update_settings",
        None,
        async {
            panic!("interrupted transport fixture");
            #[expect(
                unreachable_code,
                reason = "fixture deliberately panics before returning"
            )]
            Ok::<_, u8>(())
        },
    ));
    assert!(catch_unwind(AssertUnwindSafe(|| request.as_mut().poll(&mut cx))).is_err());
    assert_eq!(icp.timing.active.load(Ordering::SeqCst), 0);
    let events = events.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].succeeded, None);
}

#[test]
fn disabled_async_diagnostics_preserve_typed_result_without_worker_count() {
    let icp = IcpCli::new("unused", None);
    let mut cx = Context::from_waker(Waker::noop());
    let mut request = Box::pin(icp.measure_async_request(
        IcpRequestKind::AgentQuery,
        Principal::management_canister(),
        "unused",
        None,
        ready(Err::<(), _>(9_u8)),
    ));
    assert_eq!(request.as_mut().poll(&mut cx), Poll::Ready(Err(9)));
    assert_eq!(icp.timing.active.load(Ordering::SeqCst), 0);
}
