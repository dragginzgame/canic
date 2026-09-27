use super::*;
use canic_host::fleet_ensure::dto::FleetObservationStage;

#[test]
fn cache_acquisition_distinguishes_build_from_reuse_and_retains_phase() {
    use ic_testkit::artifacts::{
        ArtifactCacheOutcome, ArtifactCachePreparation, ArtifactCacheSpec, prepare_artifact_cache,
    };

    let span = Span::start("cache_qualification");
    let root = std::env::temp_dir().join(format!(
        "canic-cache-timing-{}-{}",
        std::process::id(),
        span.id
    ));
    let spec = ArtifactCacheSpec::new(&root.join("cache"), "timing", "canic/timing-test/v1")
        .with_output("probe", &root.join("probe"));
    let ArtifactCachePreparation::Build(transaction) = prepare_artifact_cache(&spec).unwrap()
    else {
        panic!("empty cache must build");
    };
    std::fs::write(transaction.output_path("probe").unwrap(), b"exact artifact").unwrap();
    let built = transaction.commit().unwrap();
    let ArtifactCachePreparation::Reused(record) = prepare_artifact_cache(&spec).unwrap() else {
        panic!("unchanged artifact must be reused");
    };
    let reused = ArtifactCacheOutcome::Reused(record);
    for (outcome, is_reused) in [(&built, false), (&reused, true)] {
        let event = serde_json::to_value(cache::CacheEvent::new("fixture", outcome)).unwrap();
        assert_eq!(event["parent_id"], span.id);
        assert_eq!(event["process_id"], std::process::id());
        assert_eq!(event["reused"], is_reused);
        assert_eq!(event["caller_build_us"].is_null(), is_reused);
        for field in [
            "coordination_lock_wait_us",
            "content_lock_wait_us",
            "namespace_lock_wait_us",
            "input_capture_us",
            "cache_lookup_us",
            "output_validation_us",
            "publication_us",
            "materialization_us",
            "total_us",
        ] {
            assert!(event[field].is_number(), "{field} must retain a duration");
        }
        artifact_cache("fixture", outcome);
    }
    assert_eq!(
        std::fs::read(reused.record().artifacts()[0].path()).unwrap(),
        b"exact artifact"
    );
    drop(reused);
    drop(built);
    std::fs::remove_dir_all(root).unwrap();
    span.finish();
}

#[test]
fn observation_failure_retains_attempts_duration_and_current_phase() {
    let span = Span::start("review");
    let timing = FleetObservationTiming {
        span_id: 1,
        parent_span_id: None,
        identity_lookup_millis: 0,
        stage: FleetObservationStage::TerminalInventory,
        parent_stage: None,
        elapsed_millis: 17,
        remote_call_attempts: 3,
        identity_lookup_attempts: 2,
        cached_read_hits: 4,
        succeeded: Some(false),
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
