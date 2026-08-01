//! Module: ops::runtime::metrics::placement_index
//!
//! Responsibility: record and snapshot low-cardinality runtime metrics for the index family.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use crate::{InternalError, InternalErrorClass, InternalErrorOrigin};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static PLACEMENT_INDEX_METRICS: RefCell<HashMap<PlacementIndexMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// PlacementIndexMetricOperation
///
/// Placement-index operation dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlacementIndexMetricOperation {
    Bind,
    Claim,
    Classify,
    CleanupStale,
    CreateInstance,
    Finalize,
    Recover,
    RecycleAbandoned,
    RepairStale,
    Resolve,
}

impl PlacementIndexMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Bind => "bind",
            Self::Claim => "claim",
            Self::Classify => "classify",
            Self::CleanupStale => "cleanup_stale",
            Self::CreateInstance => "create_instance",
            Self::Finalize => "finalize",
            Self::Recover => "recover",
            Self::RecycleAbandoned => "recycle_abandoned",
            Self::RepairStale => "repair_stale",
            Self::Resolve => "resolve",
        }
    }
}

///
/// PlacementIndexMetricOutcome
///
/// Placement-index outcome dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlacementIndexMetricOutcome {
    Completed,
    Failed,
    Skipped,
    Started,
}

impl PlacementIndexMetricOutcome {
    /// Return the stable public metrics label for this outcome.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Started => "started",
        }
    }
}

///
/// PlacementIndexMetricReason
///
/// Bounded index reason dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlacementIndexMetricReason {
    AlreadyBound,
    Claimed,
    ClaimLost,
    InvalidChild,
    InvalidState,
    ManagementCall,
    Missing,
    Ok,
    PendingCurrent,
    PendingFresh,
    PolicyDenied,
    ReleasedStale,
    ResumedPending,
    RoleMismatch,
    StaleCleanup,
    StaleRepairable,
    Unknown,
}

impl PlacementIndexMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::AlreadyBound => "already_bound",
            Self::ClaimLost => "claim_lost",
            Self::Claimed => "claimed",
            Self::InvalidChild => "invalid_child",
            Self::InvalidState => "invalid_state",
            Self::ManagementCall => "management_call",
            Self::Missing => "missing",
            Self::Ok => "ok",
            Self::PendingCurrent => "pending_current",
            Self::PendingFresh => "pending_fresh",
            Self::PolicyDenied => "policy_denied",
            Self::ReleasedStale => "released_stale",
            Self::ResumedPending => "resumed_pending",
            Self::RoleMismatch => "role_mismatch",
            Self::StaleCleanup => "stale_cleanup",
            Self::StaleRepairable => "stale_repairable",
            Self::Unknown => "unknown",
        }
    }

    /// Classify one internal error into a bounded metric reason.
    #[must_use]
    pub(crate) const fn from_error(err: &InternalError) -> Self {
        match (err.class(), err.origin()) {
            (InternalErrorClass::Infra, InternalErrorOrigin::Infra) => Self::ManagementCall,
            (InternalErrorClass::Access | InternalErrorClass::Domain, _) => Self::PolicyDenied,
            (InternalErrorClass::Ops, InternalErrorOrigin::Ops)
            | (InternalErrorClass::Invariant | InternalErrorClass::Workflow, _) => {
                Self::InvalidState
            }
            _ => Self::Unknown,
        }
    }
}

///
/// PlacementIndexMetricKey
///
/// Composite key for one low-cardinality index counter.
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct PlacementIndexMetricKey {
    pub operation: PlacementIndexMetricOperation,
    pub outcome: PlacementIndexMetricOutcome,
    pub reason: PlacementIndexMetricReason,
}

///
/// PlacementIndexMetrics
///
/// Operations-layer recorder for index placement counters.
///

pub struct PlacementIndexMetrics;

impl PlacementIndexMetrics {
    /// Record one index placement event.
    pub fn record(
        operation: PlacementIndexMetricOperation,
        outcome: PlacementIndexMetricOutcome,
        reason: PlacementIndexMetricReason,
    ) {
        PLACEMENT_INDEX_METRICS.with_borrow_mut(|counts| {
            let key = PlacementIndexMetricKey {
                operation,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current index metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(PlacementIndexMetricKey, u64)> {
        PLACEMENT_INDEX_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all index metrics.
    #[cfg(test)]
    pub fn reset() {
        PLACEMENT_INDEX_METRICS.with_borrow_mut(HashMap::clear);
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<PlacementIndexMetricKey, u64> {
        PlacementIndexMetrics::snapshot().into_iter().collect()
    }

    // Verify index metrics accumulate by operation, outcome, and reason.
    #[test]
    fn index_metrics_accumulate_by_operation_outcome_and_reason() {
        PlacementIndexMetrics::reset();

        PlacementIndexMetrics::record(
            PlacementIndexMetricOperation::Resolve,
            PlacementIndexMetricOutcome::Started,
            PlacementIndexMetricReason::Ok,
        );
        PlacementIndexMetrics::record(
            PlacementIndexMetricOperation::Classify,
            PlacementIndexMetricOutcome::Completed,
            PlacementIndexMetricReason::PendingFresh,
        );
        PlacementIndexMetrics::record(
            PlacementIndexMetricOperation::Classify,
            PlacementIndexMetricOutcome::Completed,
            PlacementIndexMetricReason::PendingFresh,
        );

        let map = snapshot_map();

        assert_eq!(
            map.get(&PlacementIndexMetricKey {
                operation: PlacementIndexMetricOperation::Resolve,
                outcome: PlacementIndexMetricOutcome::Started,
                reason: PlacementIndexMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&PlacementIndexMetricKey {
                operation: PlacementIndexMetricOperation::Classify,
                outcome: PlacementIndexMetricOutcome::Completed,
                reason: PlacementIndexMetricReason::PendingFresh,
            }),
            Some(&2)
        );
    }
}
