//! Module: ops::runtime::metrics::intent
//!
//! Responsibility: record and snapshot low-cardinality runtime metrics for the intent family.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static INTENT_METRICS: RefCell<HashMap<IntentMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// IntentMetricSurface
///
/// Intent metric surface dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum IntentMetricSurface {
    Cleanup,
    Local,
    Pool,
    ReceiptBacked,
}

impl IntentMetricSurface {
    /// Return the stable public metrics label for this surface.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Cleanup => "cleanup",
            Self::Local => "local",
            Self::Pool => "pool",
            Self::ReceiptBacked => "receipt_backed",
        }
    }
}

///
/// IntentMetricOperation
///
/// Intent metric operation dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum IntentMetricOperation {
    Abort,
    CapacityCheck,
    Cleanup,
    Commit,
    Reserve,
}

impl IntentMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Abort => "abort",
            Self::CapacityCheck => "capacity_check",
            Self::Cleanup => "cleanup",
            Self::Commit => "commit",
            Self::Reserve => "reserve",
        }
    }
}

///
/// IntentMetricOutcome
///
/// Intent metric outcome dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum IntentMetricOutcome {
    Completed,
    Failed,
}

impl IntentMetricOutcome {
    /// Return the stable public metrics label for this outcome.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

///
/// IntentMetricReason
///
/// Bounded intent reason dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum IntentMetricReason {
    Capacity,
    Expired,
    NoExpired,
    Ok,
    Overflow,
    StorageFailed,
}

impl IntentMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Capacity => "capacity",
            Self::Expired => "expired",
            Self::NoExpired => "no_expired",
            Self::Ok => "ok",
            Self::Overflow => "overflow",
            Self::StorageFailed => "storage_failed",
        }
    }
}

///
/// IntentMetricKey
///
/// Composite key for one low-cardinality intent counter.
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct IntentMetricKey {
    pub surface: IntentMetricSurface,
    pub operation: IntentMetricOperation,
    pub outcome: IntentMetricOutcome,
    pub reason: IntentMetricReason,
}

///
/// IntentMetrics
///
/// Operations-layer recorder for intent reservation counters.
///

pub struct IntentMetrics;

impl IntentMetrics {
    /// Record one intent event.
    pub fn record(
        surface: IntentMetricSurface,
        operation: IntentMetricOperation,
        outcome: IntentMetricOutcome,
        reason: IntentMetricReason,
    ) {
        INTENT_METRICS.with_borrow_mut(|counts| {
            let key = IntentMetricKey {
                surface,
                operation,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current intent metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(IntentMetricKey, u64)> {
        INTENT_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all intent metrics.
    #[cfg(test)]
    pub fn reset() {
        INTENT_METRICS.with_borrow_mut(HashMap::clear);
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<IntentMetricKey, u64> {
        IntentMetrics::snapshot().into_iter().collect()
    }

    #[test]
    fn intent_metrics_accumulate_by_surface_operation_outcome_and_reason() {
        IntentMetrics::reset();

        IntentMetrics::record(
            IntentMetricSurface::Local,
            IntentMetricOperation::Reserve,
            IntentMetricOutcome::Completed,
            IntentMetricReason::Ok,
        );
        IntentMetrics::record(
            IntentMetricSurface::Local,
            IntentMetricOperation::Reserve,
            IntentMetricOutcome::Completed,
            IntentMetricReason::Ok,
        );
        IntentMetrics::record(
            IntentMetricSurface::Cleanup,
            IntentMetricOperation::Abort,
            IntentMetricOutcome::Completed,
            IntentMetricReason::Expired,
        );

        let map = snapshot_map();
        assert_eq!(
            map.get(&IntentMetricKey {
                surface: IntentMetricSurface::Local,
                operation: IntentMetricOperation::Reserve,
                outcome: IntentMetricOutcome::Completed,
                reason: IntentMetricReason::Ok,
            }),
            Some(&2)
        );
        assert_eq!(
            map.get(&IntentMetricKey {
                surface: IntentMetricSurface::Cleanup,
                operation: IntentMetricOperation::Abort,
                outcome: IntentMetricOutcome::Completed,
                reason: IntentMetricReason::Expired,
            }),
            Some(&1)
        );
    }
}
