//! Module: ops::runtime::metrics::placement_binding
//!
//! Responsibility: record and snapshot low-cardinality runtime metrics for the binding family.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use crate::{InternalError, InternalErrorClass, InternalErrorOrigin};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static PLACEMENT_BINDING_METRICS: RefCell<HashMap<PlacementBindingMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// PlacementBindingMetricOperation
///
/// Placement-binding operation dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlacementBindingMetricOperation {
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

impl PlacementBindingMetricOperation {
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
/// PlacementBindingMetricOutcome
///
/// Placement-binding outcome dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlacementBindingMetricOutcome {
    Completed,
    Failed,
    Skipped,
    Started,
}

impl PlacementBindingMetricOutcome {
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
/// PlacementBindingMetricReason
///
/// Bounded binding reason dimension used by public metrics projection.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PlacementBindingMetricReason {
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
    RegistryMissing,
    ReleasedStale,
    ResumedPending,
    RoleMismatch,
    StaleCleanup,
    StaleRepairable,
    Unknown,
}

impl PlacementBindingMetricReason {
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
            Self::RegistryMissing => "registry_missing",
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
/// PlacementBindingMetricKey
///
/// Composite key for one low-cardinality binding counter.
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct PlacementBindingMetricKey {
    pub operation: PlacementBindingMetricOperation,
    pub outcome: PlacementBindingMetricOutcome,
    pub reason: PlacementBindingMetricReason,
}

///
/// PlacementBindingMetrics
///
/// Operations-layer recorder for binding placement counters.
///

pub struct PlacementBindingMetrics;

impl PlacementBindingMetrics {
    /// Record one binding placement event.
    pub fn record(
        operation: PlacementBindingMetricOperation,
        outcome: PlacementBindingMetricOutcome,
        reason: PlacementBindingMetricReason,
    ) {
        PLACEMENT_BINDING_METRICS.with_borrow_mut(|counts| {
            let key = PlacementBindingMetricKey {
                operation,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current binding metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(PlacementBindingMetricKey, u64)> {
        PLACEMENT_BINDING_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all binding metrics.
    #[cfg(test)]
    pub fn reset() {
        PLACEMENT_BINDING_METRICS.with_borrow_mut(HashMap::clear);
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<PlacementBindingMetricKey, u64> {
        PlacementBindingMetrics::snapshot().into_iter().collect()
    }

    // Verify binding metrics accumulate by operation, outcome, and reason.
    #[test]
    fn binding_metrics_accumulate_by_operation_outcome_and_reason() {
        PlacementBindingMetrics::reset();

        PlacementBindingMetrics::record(
            PlacementBindingMetricOperation::Resolve,
            PlacementBindingMetricOutcome::Started,
            PlacementBindingMetricReason::Ok,
        );
        PlacementBindingMetrics::record(
            PlacementBindingMetricOperation::Classify,
            PlacementBindingMetricOutcome::Completed,
            PlacementBindingMetricReason::PendingFresh,
        );
        PlacementBindingMetrics::record(
            PlacementBindingMetricOperation::Classify,
            PlacementBindingMetricOutcome::Completed,
            PlacementBindingMetricReason::PendingFresh,
        );

        let map = snapshot_map();

        assert_eq!(
            map.get(&PlacementBindingMetricKey {
                operation: PlacementBindingMetricOperation::Resolve,
                outcome: PlacementBindingMetricOutcome::Started,
                reason: PlacementBindingMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&PlacementBindingMetricKey {
                operation: PlacementBindingMetricOperation::Classify,
                outcome: PlacementBindingMetricOutcome::Completed,
                reason: PlacementBindingMetricReason::PendingFresh,
            }),
            Some(&2)
        );
    }
}
