use crate::{InternalError, InternalErrorClass, InternalErrorOrigin};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static DIRECTORY_METRICS: RefCell<HashMap<DirectoryMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// DirectoryMetricOperation
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum DirectoryMetricOperation {
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

impl DirectoryMetricOperation {
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
/// DirectoryMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum DirectoryMetricOutcome {
    Completed,
    Failed,
    Skipped,
    Started,
}

impl DirectoryMetricOutcome {
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
/// DirectoryMetricReason
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum DirectoryMetricReason {
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
    RoleMismatch,
    StaleCleanup,
    StaleRepairable,
    Unknown,
}

impl DirectoryMetricReason {
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
/// DirectoryMetricKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct DirectoryMetricKey {
    pub operation: DirectoryMetricOperation,
    pub outcome: DirectoryMetricOutcome,
    pub reason: DirectoryMetricReason,
}

///
/// DirectoryMetrics
///

pub struct DirectoryMetrics;

impl DirectoryMetrics {
    /// Record one directory placement event.
    pub fn record(
        operation: DirectoryMetricOperation,
        outcome: DirectoryMetricOutcome,
        reason: DirectoryMetricReason,
    ) {
        DIRECTORY_METRICS.with_borrow_mut(|counts| {
            let key = DirectoryMetricKey {
                operation,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current directory metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(DirectoryMetricKey, u64)> {
        DIRECTORY_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all directory metrics.
    #[cfg(test)]
    pub fn reset() {
        DIRECTORY_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<DirectoryMetricKey, u64> {
        DirectoryMetrics::snapshot().into_iter().collect()
    }

    // Verify directory metrics accumulate by operation, outcome, and reason.
    #[test]
    fn directory_metrics_accumulate_by_operation_outcome_and_reason() {
        DirectoryMetrics::reset();

        DirectoryMetrics::record(
            DirectoryMetricOperation::Resolve,
            DirectoryMetricOutcome::Started,
            DirectoryMetricReason::Ok,
        );
        DirectoryMetrics::record(
            DirectoryMetricOperation::Classify,
            DirectoryMetricOutcome::Completed,
            DirectoryMetricReason::PendingFresh,
        );
        DirectoryMetrics::record(
            DirectoryMetricOperation::Classify,
            DirectoryMetricOutcome::Completed,
            DirectoryMetricReason::PendingFresh,
        );

        let map = snapshot_map();

        assert_eq!(
            map.get(&DirectoryMetricKey {
                operation: DirectoryMetricOperation::Resolve,
                outcome: DirectoryMetricOutcome::Started,
                reason: DirectoryMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&DirectoryMetricKey {
                operation: DirectoryMetricOperation::Classify,
                outcome: DirectoryMetricOutcome::Completed,
                reason: DirectoryMetricReason::PendingFresh,
            }),
            Some(&2)
        );
    }
}
