use super::*;
use canic_host::fleet_ensure::dto::FleetObservationStage;

#[test]
fn observation_failure_retains_attempts_duration_and_current_phase() {
    let span = Span::start("review");
    let timing = FleetObservationTiming {
        stage: FleetObservationStage::TerminalInventory,
        elapsed_millis: 17,
        remote_call_attempts: 3,
        succeeded: false,
    };
    let event = ObservationEvent::new(timing.clone());
    assert_eq!(event.parent_id, Some(span.id));
    assert_eq!(event.observation, timing);
    let value = serde_json::to_value(&event).unwrap();
    assert_eq!(value["observation"]["succeeded"], false);
    observation(timing);
    span.finish();
}

#[test]
fn nested_phases_retain_parentage_and_monotonic_durations() {
    let root = Span::start("journey");
    let child = Span::start("artifacts");
    assert_eq!(child.parent_id, Some(root.id));
    let start = child.record(State::Started);
    let end = child.record(State::Completed);
    assert!(end.elapsed_us >= start.elapsed_us);
    assert_eq!(start.span_id, end.span_id);
    let sibling = child.next("setup");
    assert_eq!(sibling.parent_id, Some(root.id));
    assert_ne!(sibling.id, start.span_id);
    sibling.finish();
    assert_eq!(ACTIVE.with_borrow(Clone::clone), vec![root.id]);
    root.finish();
    assert!(ACTIVE.with_borrow(Vec::is_empty));
}

#[test]
fn dropped_and_panicking_phases_leave_no_parent_for_the_next_case() {
    drop(Span::start("interrupted"));
    assert!(ACTIVE.with_borrow(Vec::is_empty));
    let result = std::panic::catch_unwind(|| {
        let _root = Span::start("failed_case");
        let _child = Span::start("failed_phase");
        panic!("fixture failure");
    });
    assert!(result.is_err());
    let next = Span::start("next_case");
    assert_eq!(next.parent_id, None);
    let value = serde_json::to_value(next.record(State::Incomplete)).unwrap();
    assert_eq!(value["state"], "incomplete");
    assert_eq!(value["schema_version"], 1);
    next.finish();
}
