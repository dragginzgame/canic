//! Module: ops::runtime::metrics::lifecycle
//!
//! Responsibility: record and snapshot low-cardinality runtime metrics for the lifecycle family.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use std::{cell::RefCell, collections::HashMap};

pub use crate::domain::metrics::{
    LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricStage,
};

thread_local! {
    static LIFECYCLE_METRICS: RefCell<HashMap<LifecycleMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// LifecycleMetricKey
///
/// Composite key for one low-cardinality lifecycle counter.
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct LifecycleMetricKey {
    pub phase: LifecycleMetricPhase,
    pub role: LifecycleMetricRole,
    pub stage: LifecycleMetricStage,
    pub outcome: LifecycleMetricOutcome,
}

///
/// LifecycleMetrics
///
/// Operations-layer recorder for lifecycle runtime counters.
///

pub struct LifecycleMetrics;

impl LifecycleMetrics {
    /// Record one lifecycle stage event.
    pub fn record(
        phase: LifecycleMetricPhase,
        role: LifecycleMetricRole,
        stage: LifecycleMetricStage,
        outcome: LifecycleMetricOutcome,
    ) {
        LIFECYCLE_METRICS.with_borrow_mut(|counts| {
            let key = LifecycleMetricKey {
                phase,
                role,
                stage,
                outcome,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current lifecycle metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(LifecycleMetricKey, u64)> {
        LIFECYCLE_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all lifecycle metrics.
    #[cfg(test)]
    pub fn reset() {
        LIFECYCLE_METRICS.with_borrow_mut(HashMap::clear);
    }
}
