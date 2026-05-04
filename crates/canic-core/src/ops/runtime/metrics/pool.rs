use crate::{
    InternalError, InternalErrorClass, InternalErrorOrigin, domain::policy::pool::PoolPolicyError,
};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static POOL_METRICS: RefCell<HashMap<PoolMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// PoolMetricOperation
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PoolMetricOperation {
    CreateEmpty,
    ImportImmediate,
    ImportQueued,
    Recycle,
    Reset,
    Scheduler,
    SelectReady,
}

impl PoolMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::CreateEmpty => "create_empty",
            Self::ImportImmediate => "import_immediate",
            Self::ImportQueued => "import_queued",
            Self::Recycle => "recycle",
            Self::Reset => "reset",
            Self::Scheduler => "scheduler",
            Self::SelectReady => "select_ready",
        }
    }
}

///
/// PoolMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PoolMetricOutcome {
    Completed,
    Failed,
    Requeued,
    Scheduled,
    Skipped,
    Started,
}

impl PoolMetricOutcome {
    /// Return the stable public metrics label for this outcome.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Requeued => "requeued",
            Self::Scheduled => "scheduled",
            Self::Skipped => "skipped",
            Self::Started => "started",
        }
    }
}

///
/// PoolMetricReason
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum PoolMetricReason {
    AlreadyPresent,
    Empty,
    FailedEntry,
    InProgress,
    InvalidState,
    ManagementCall,
    NonImportableLocal,
    NotFound,
    Ok,
    PolicyDenied,
    RegisteredInSubnet,
    Unknown,
}

impl PoolMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::AlreadyPresent => "already_present",
            Self::Empty => "empty",
            Self::FailedEntry => "failed_entry",
            Self::InProgress => "in_progress",
            Self::InvalidState => "invalid_state",
            Self::ManagementCall => "management_call",
            Self::NonImportableLocal => "non_importable_local",
            Self::NotFound => "not_found",
            Self::Ok => "ok",
            Self::PolicyDenied => "policy_denied",
            Self::RegisteredInSubnet => "registered_in_subnet",
            Self::Unknown => "unknown",
        }
    }

    /// Classify one internal error into a bounded metric reason.
    #[must_use]
    pub(crate) const fn from_error(err: &InternalError) -> Self {
        match (err.class(), err.origin()) {
            (InternalErrorClass::Infra, InternalErrorOrigin::Infra) => Self::ManagementCall,
            (InternalErrorClass::Access | InternalErrorClass::Domain, _) => Self::PolicyDenied,
            (
                InternalErrorClass::Invariant
                | InternalErrorClass::Ops
                | InternalErrorClass::Workflow,
                _,
            ) => Self::InvalidState,
            _ => Self::Unknown,
        }
    }

    /// Classify one pool policy rejection into a bounded metric reason.
    #[must_use]
    pub(crate) const fn from_policy(err: &PoolPolicyError) -> Self {
        match err {
            PoolPolicyError::RegisteredInSubnet(_) => Self::RegisteredInSubnet,
            PoolPolicyError::NonImportableOnLocal { .. } => Self::NonImportableLocal,
            PoolPolicyError::NotRegisteredInSubnet(_) => Self::NotFound,
            PoolPolicyError::NotAuthorized => Self::PolicyDenied,
        }
    }
}

///
/// PoolMetricKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct PoolMetricKey {
    pub operation: PoolMetricOperation,
    pub outcome: PoolMetricOutcome,
    pub reason: PoolMetricReason,
}

///
/// PoolMetrics
///

pub struct PoolMetrics;

impl PoolMetrics {
    /// Record one pool operation event.
    pub fn record(
        operation: PoolMetricOperation,
        outcome: PoolMetricOutcome,
        reason: PoolMetricReason,
    ) {
        POOL_METRICS.with_borrow_mut(|counts| {
            let key = PoolMetricKey {
                operation,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current pool metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(PoolMetricKey, u64)> {
        POOL_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all pool metrics.
    #[cfg(test)]
    pub fn reset() {
        POOL_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<PoolMetricKey, u64> {
        PoolMetrics::snapshot().into_iter().collect()
    }

    // Verify pool metrics accumulate by operation, outcome, and reason.
    #[test]
    fn pool_metrics_accumulate_by_operation_outcome_and_reason() {
        PoolMetrics::reset();

        PoolMetrics::record(
            PoolMetricOperation::Reset,
            PoolMetricOutcome::Started,
            PoolMetricReason::Ok,
        );
        PoolMetrics::record(
            PoolMetricOperation::ImportQueued,
            PoolMetricOutcome::Skipped,
            PoolMetricReason::AlreadyPresent,
        );
        PoolMetrics::record(
            PoolMetricOperation::ImportQueued,
            PoolMetricOutcome::Skipped,
            PoolMetricReason::AlreadyPresent,
        );

        let map = snapshot_map();

        assert_eq!(
            map.get(&PoolMetricKey {
                operation: PoolMetricOperation::Reset,
                outcome: PoolMetricOutcome::Started,
                reason: PoolMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&PoolMetricKey {
                operation: PoolMetricOperation::ImportQueued,
                outcome: PoolMetricOutcome::Skipped,
                reason: PoolMetricReason::AlreadyPresent,
            }),
            Some(&2)
        );
    }
}
