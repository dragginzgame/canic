//! Module: ops::runtime::metrics::platform_call
//!
//! Responsibility: record and snapshot low-cardinality runtime metrics for the platform_call family.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use ic_metrics::MeasurementCounter;
use std::{cell::RefCell, collections::HashMap};

pub use crate::domain::metrics::{
    PlatformCallMetricMode, PlatformCallMetricOutcome, PlatformCallMetricReason,
    PlatformCallMetricSurface,
};

thread_local! {
    static PLATFORM_CALL_METRICS: RefCell<HashMap<PlatformCallMetricKey, MeasurementCounter>> =
        RefCell::new(HashMap::new());
}

///
/// PlatformCallMetricKey
///
/// Composite key for one low-cardinality platform call counter.
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct PlatformCallMetricKey {
    pub surface: PlatformCallMetricSurface,
    pub mode: PlatformCallMetricMode,
    pub outcome: PlatformCallMetricOutcome,
    pub reason: PlatformCallMetricReason,
}

///
/// PlatformCallMetrics
///
/// Operations-layer recorder for platform call counters.
///

pub struct PlatformCallMetrics;

impl PlatformCallMetrics {
    /// Bound source visits before allocating an optional anonymous aggregate.
    pub(crate) fn bounded_snapshot(
        limit: usize,
    ) -> Result<Vec<(PlatformCallMetricKey, MeasurementCounter)>, crate::InternalError> {
        PLATFORM_CALL_METRICS.with_borrow(|counts| {
            if counts.len() > limit {
                return Err(crate::InternalError::invalid_input());
            }
            Ok(counts
                .iter()
                .take(limit)
                .map(|(key, count)| (*key, *count))
                .collect())
        })
    }

    /// Record one platform call event.
    pub fn record(
        surface: PlatformCallMetricSurface,
        mode: PlatformCallMetricMode,
        outcome: PlatformCallMetricOutcome,
        reason: PlatformCallMetricReason,
    ) {
        PLATFORM_CALL_METRICS.with_borrow_mut(|counts| {
            let key = PlatformCallMetricKey {
                surface,
                mode,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_default();
            entry.add(1);
        });
    }

    /// Snapshot the current platform call metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(PlatformCallMetricKey, MeasurementCounter)> {
        PLATFORM_CALL_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all platform call metrics.
    #[cfg(test)]
    pub fn reset() {
        PLATFORM_CALL_METRICS.with_borrow_mut(HashMap::clear);
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<PlatformCallMetricKey, u128> {
        PlatformCallMetrics::snapshot()
            .into_iter()
            .map(|(key, count)| (key, count.exact_total().unwrap()))
            .collect()
    }

    #[test]
    fn platform_call_metrics_accumulate_by_surface_mode_outcome_and_reason() {
        PlatformCallMetrics::reset();

        PlatformCallMetrics::record(
            PlatformCallMetricSurface::Generic,
            PlatformCallMetricMode::BoundedWait,
            PlatformCallMetricOutcome::Started,
            PlatformCallMetricReason::Ok,
        );
        PlatformCallMetrics::record(
            PlatformCallMetricSurface::Generic,
            PlatformCallMetricMode::BoundedWait,
            PlatformCallMetricOutcome::Started,
            PlatformCallMetricReason::Ok,
        );
        PlatformCallMetrics::record(
            PlatformCallMetricSurface::Management,
            PlatformCallMetricMode::Update,
            PlatformCallMetricOutcome::Failed,
            PlatformCallMetricReason::Infra,
        );

        let map = snapshot_map();
        assert_eq!(
            map.get(&PlatformCallMetricKey {
                surface: PlatformCallMetricSurface::Generic,
                mode: PlatformCallMetricMode::BoundedWait,
                outcome: PlatformCallMetricOutcome::Started,
                reason: PlatformCallMetricReason::Ok,
            }),
            Some(&2)
        );
        assert_eq!(
            map.get(&PlatformCallMetricKey {
                surface: PlatformCallMetricSurface::Management,
                mode: PlatformCallMetricMode::Update,
                outcome: PlatformCallMetricOutcome::Failed,
                reason: PlatformCallMetricReason::Infra,
            }),
            Some(&1)
        );
    }
}
