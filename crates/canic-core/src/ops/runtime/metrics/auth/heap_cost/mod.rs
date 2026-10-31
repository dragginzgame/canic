//! Measure the real Auth pilot table, including capacity and snapshot temporaries.
//! Native allocator requests are evidence for this target, not Wasm allocator overhead.

use super::{
    AUTH_METRICS, AuthMetricOperation, AuthMetricOutcome, AuthMetricReason, AuthMetricSurface,
    AuthMetrics, attestation, sessions,
};
use crate::test::allocation::measure;
use std::collections::HashMap;

fn record_pilot() {
    attestation::record_attestation_verify_failed();
    attestation::record_attestation_epoch_rejected();
    sessions::record_application_session_cleanup(1);
    sessions::record_application_session_generation_invalidation();
}

#[test]
fn complete_pilot_table_and_projection_have_measured_allocation_cost() {
    AUTH_METRICS.with_borrow_mut(|table| *table = HashMap::new());
    assert!(AuthMetrics::snapshot().is_empty());
    let ((), cold) = measure(record_pilot);
    assert!(cold.retained_bytes > 0);
    let ((), warm) = measure(|| {
        for _ in 0..128 {
            record_pilot();
        }
    });
    assert_eq!(warm.allocations, 0);
    assert_eq!(warm.retained_bytes, 0);
    let ((), snapshot) = measure(|| {
        let rows = AuthMetrics::snapshot();
        let expected = [
            (
                AuthMetricSurface::Attestation,
                AuthMetricOperation::Verify,
                AuthMetricOutcome::Failed,
                AuthMetricReason::VerifyFailed,
            ),
            (
                AuthMetricSurface::Attestation,
                AuthMetricOperation::Verify,
                AuthMetricOutcome::Failed,
                AuthMetricReason::EpochRejected,
            ),
            (
                AuthMetricSurface::ApplicationSession,
                AuthMetricOperation::Cleanup,
                AuthMetricOutcome::Completed,
                AuthMetricReason::Expired,
            ),
            (
                AuthMetricSurface::ApplicationSession,
                AuthMetricOperation::GenerationInvalidation,
                AuthMetricOutcome::Completed,
                AuthMetricReason::GenerationAdvanced,
            ),
        ];
        assert_eq!(rows.len(), expected.len());
        for (surface, operation, outcome, reason) in expected {
            let (_, count) = rows
                .iter()
                .find(|(key, _)| {
                    key.surface == surface
                        && key.operation == operation
                        && key.outcome == outcome
                        && key.reason == reason
                })
                .unwrap();
            assert_eq!(count.exact_total().unwrap(), 129);
        }
    });
    assert_eq!(snapshot.retained_bytes, 0);
    assert!(snapshot.peak_bytes > 0);
    let ((), reset) = measure(|| AUTH_METRICS.with_borrow_mut(|table| *table = HashMap::new()));
    assert_eq!(reset.retained_bytes, -cold.retained_bytes);
    assert!(AuthMetrics::snapshot().is_empty());
    eprintln!(
        "Auth pilot native {}-bit: cold {cold:?}; 512 warm records {warm:?}; complete snapshot {snapshot:?}; reset {reset:?}",
        usize::BITS
    );
}
