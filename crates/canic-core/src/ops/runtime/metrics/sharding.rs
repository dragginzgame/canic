use crate::{
    InternalError, InternalErrorClass, InternalErrorOrigin,
    domain::policy::placement::sharding::CreateBlockedReason,
};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static SHARDING_METRICS: RefCell<HashMap<ShardingMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// ShardingMetricOperation
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ShardingMetricOperation {
    Assign,
    AssignKey,
    BootstrapActive,
    BootstrapConfig,
    BootstrapPool,
    CreateShard,
    PlanAssign,
}

impl ShardingMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Assign => "assign",
            Self::AssignKey => "assign_key",
            Self::BootstrapActive => "bootstrap_active",
            Self::BootstrapConfig => "bootstrap_config",
            Self::BootstrapPool => "bootstrap_pool",
            Self::CreateShard => "create_shard",
            Self::PlanAssign => "plan_assign",
        }
    }
}

///
/// ShardingMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ShardingMetricOutcome {
    Completed,
    Failed,
    Skipped,
    Started,
}

impl ShardingMetricOutcome {
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
/// ShardingMetricReason
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ShardingMetricReason {
    AlreadyAssigned,
    CreateAllowed,
    ExistingCapacity,
    InvalidState,
    ManagementCall,
    NoFreeSlots,
    NoInitialShards,
    Ok,
    PolicyDenied,
    PoolAtCapacity,
    ShardingDisabled,
    TargetSatisfied,
    Unknown,
}

impl ShardingMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::AlreadyAssigned => "already_assigned",
            Self::CreateAllowed => "create_allowed",
            Self::ExistingCapacity => "existing_capacity",
            Self::InvalidState => "invalid_state",
            Self::ManagementCall => "management_call",
            Self::NoFreeSlots => "no_free_slots",
            Self::NoInitialShards => "no_initial_shards",
            Self::Ok => "ok",
            Self::PolicyDenied => "policy_denied",
            Self::PoolAtCapacity => "pool_at_capacity",
            Self::ShardingDisabled => "sharding_disabled",
            Self::TargetSatisfied => "target_satisfied",
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

    /// Classify one sharding policy block reason into a bounded metric reason.
    #[must_use]
    pub(crate) const fn from_create_blocked_reason(reason: &CreateBlockedReason) -> Self {
        match reason {
            CreateBlockedReason::NoFreeSlots => Self::NoFreeSlots,
            CreateBlockedReason::PoolAtCapacity => Self::PoolAtCapacity,
            CreateBlockedReason::PolicyViolation(_) => Self::PolicyDenied,
        }
    }
}

///
/// ShardingMetricKey
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct ShardingMetricKey {
    pub operation: ShardingMetricOperation,
    pub outcome: ShardingMetricOutcome,
    pub reason: ShardingMetricReason,
}

///
/// ShardingMetrics
///

pub struct ShardingMetrics;

impl ShardingMetrics {
    /// Record one sharding placement event.
    pub fn record(
        operation: ShardingMetricOperation,
        outcome: ShardingMetricOutcome,
        reason: ShardingMetricReason,
    ) {
        SHARDING_METRICS.with_borrow_mut(|counts| {
            let key = ShardingMetricKey {
                operation,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current sharding metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(ShardingMetricKey, u64)> {
        SHARDING_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all sharding metrics.
    #[cfg(test)]
    pub fn reset() {
        SHARDING_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<ShardingMetricKey, u64> {
        ShardingMetrics::snapshot().into_iter().collect()
    }

    // Verify sharding metrics accumulate by operation, outcome, and reason.
    #[test]
    fn sharding_metrics_accumulate_by_operation_outcome_and_reason() {
        ShardingMetrics::reset();

        ShardingMetrics::record(
            ShardingMetricOperation::PlanAssign,
            ShardingMetricOutcome::Completed,
            ShardingMetricReason::ExistingCapacity,
        );
        ShardingMetrics::record(
            ShardingMetricOperation::BootstrapPool,
            ShardingMetricOutcome::Skipped,
            ShardingMetricReason::TargetSatisfied,
        );
        ShardingMetrics::record(
            ShardingMetricOperation::BootstrapPool,
            ShardingMetricOutcome::Skipped,
            ShardingMetricReason::TargetSatisfied,
        );

        let map = snapshot_map();

        assert_eq!(
            map.get(&ShardingMetricKey {
                operation: ShardingMetricOperation::PlanAssign,
                outcome: ShardingMetricOutcome::Completed,
                reason: ShardingMetricReason::ExistingCapacity,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&ShardingMetricKey {
                operation: ShardingMetricOperation::BootstrapPool,
                outcome: ShardingMetricOutcome::Skipped,
                reason: ShardingMetricReason::TargetSatisfied,
            }),
            Some(&2)
        );
    }
}
