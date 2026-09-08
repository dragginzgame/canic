//! Module: workflow::metrics::publication
//!
//! Responsibility: sample explicitly selected families and expose public cache projections.
//! Does not own: authorization, timer registration, or snapshot records.
//! Boundary: trusted local updates sample; public queries only project cached values.

use crate::{
    InternalError,
    dto::public_status::{PublicHealth, PublicMetric, PublicMetricsRequest, PublicMetricsSnapshot},
    ops::{ic::IcOps, runtime::public_metrics::PublicMetricsOps},
};

/// Public snapshot reads and explicit update-side publication.
pub struct PublicMetricsWorkflow;

impl PublicMetricsWorkflow {
    #[must_use]
    pub fn health() -> PublicHealth {
        PublicMetricsOps::health()
    }
    #[must_use]
    pub fn read(request: PublicMetricsRequest) -> PublicMetricsSnapshot {
        PublicMetricsOps::read(request)
    }
    pub fn record_application(metrics: Vec<PublicMetric>) -> Result<(), InternalError> {
        PublicMetricsOps::record_application(metrics)
    }
    pub fn sample() -> Result<(), InternalError> {
        let families = PublicMetricsOps::enabled();
        if families.is_empty() {
            return Ok(());
        }
        let now = IcOps::now_nanos();
        Self::sample_selected(families, now)
    }

    fn sample_selected(
        families: std::collections::BTreeSet<crate::domain::public_metrics::PublicMetricFamily>,
        now: u64,
    ) -> Result<(), InternalError> {
        let mut failure = None;
        for family in families {
            if let Err(error) = PublicMetricsOps::sample_family(family, now) {
                failure.get_or_insert(error);
            }
        }
        failure.map_or(Ok(()), Err)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::public_metrics::PublicMetricFamily,
        model::public_metrics::{PublicMetricSample, PublicMetricsCache},
    };
    use std::collections::BTreeSet;

    #[test]
    fn rejected_performance_does_not_starve_selected_occupancy() {
        crate::perf::reset();
        let performance = PublicMetricFamily::Performance;
        PublicMetricsCache::replace(
            performance,
            10,
            vec![PublicMetricSample {
                name: "prior".into(),
                canister_id: None,
                value: 7,
                unit: "instructions".into(),
            }],
        )
        .unwrap();
        crate::perf::record_checkpoint(&"a".repeat(128), "accepted_checkpoint", 42);
        let result = PublicMetricsWorkflow::sample_selected(
            BTreeSet::from([performance, PublicMetricFamily::ShardOccupancy]),
            20,
        );
        assert_eq!(
            result.unwrap_err().code(),
            crate::diagnostics::codes::REQUEST_INVALID
        );
        let retained = PublicMetricsCache::snapshot(performance).unwrap();
        assert_eq!(retained.sampled_at_ns, 10);
        assert_eq!(retained.metrics[0].value, 7);
        assert_eq!(
            PublicMetricsCache::snapshot(PublicMetricFamily::ShardOccupancy)
                .unwrap()
                .sampled_at_ns,
            20
        );
        crate::perf::reset();
        PublicMetricsWorkflow::sample_selected(BTreeSet::from([performance]), 30).unwrap();
        assert_eq!(
            PublicMetricsCache::snapshot(performance)
                .unwrap()
                .sampled_at_ns,
            30
        );
    }
}
