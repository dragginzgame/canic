use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static INTENT_METRICS: RefCell<HashMap<IntentMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// IntentMetricSurface
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum IntentMetricSurface {
    Call,
    Cleanup,
    Pool,
}

impl IntentMetricSurface {
    /// Return the stable public metrics label for this surface.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Call => "call",
            Self::Cleanup => "cleanup",
            Self::Pool => "pool",
        }
    }
}

///
/// IntentMetricOperation
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

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum IntentMetricReason {
    Capacity,
    Expired,
    Idle,
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
            Self::Idle => "idle",
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

///
/// TESTS
///

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
            IntentMetricSurface::Call,
            IntentMetricOperation::Reserve,
            IntentMetricOutcome::Completed,
            IntentMetricReason::Ok,
        );
        IntentMetrics::record(
            IntentMetricSurface::Call,
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
                surface: IntentMetricSurface::Call,
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
