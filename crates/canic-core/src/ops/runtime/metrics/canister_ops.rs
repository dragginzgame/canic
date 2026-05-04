use crate::{InternalError, InternalErrorClass, InternalErrorOrigin, ids::CanisterRole};
use std::{cell::RefCell, collections::HashMap};

const UNSCOPED_ROLE_LABEL: &str = "unscoped";
const UNKNOWN_ROLE_LABEL: &str = "unknown";

thread_local! {
    static CANISTER_OPS_METRICS: RefCell<HashMap<CanisterOpsMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// CanisterOpsMetricOperation
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum CanisterOpsMetricOperation {
    Create,
    Delete,
    Install,
    Reinstall,
    Restore,
    Snapshot,
    Upgrade,
}

impl CanisterOpsMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Delete => "delete",
            Self::Install => "install",
            Self::Reinstall => "reinstall",
            Self::Restore => "restore",
            Self::Snapshot => "snapshot",
            Self::Upgrade => "upgrade",
        }
    }
}

///
/// CanisterOpsMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum CanisterOpsMetricOutcome {
    Completed,
    Failed,
    Skipped,
    Started,
}

impl CanisterOpsMetricOutcome {
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
/// CanisterOpsMetricReason
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum CanisterOpsMetricReason {
    AlreadyExists,
    Cycles,
    InvalidState,
    ManagementCall,
    MissingWasm,
    NewAllocation,
    NotFound,
    Ok,
    PolicyDenied,
    PoolReuse,
    PoolTopup,
    StatePropagation,
    Topology,
    TopologyPropagation,
    Unknown,
}

impl CanisterOpsMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::AlreadyExists => "already_exists",
            Self::NewAllocation => "new_allocation",
            Self::Cycles => "cycles",
            Self::InvalidState => "invalid_state",
            Self::ManagementCall => "management_call",
            Self::MissingWasm => "missing_wasm",
            Self::NotFound => "not_found",
            Self::Ok => "ok",
            Self::PolicyDenied => "policy_denied",
            Self::PoolReuse => "pool_reuse",
            Self::PoolTopup => "pool_topup",
            Self::StatePropagation => "state_propagation",
            Self::Topology => "topology",
            Self::TopologyPropagation => "topology_propagation",
            Self::Unknown => "unknown",
        }
    }

    /// Classify one internal error into a bounded metric reason.
    #[must_use]
    pub(crate) const fn from_error(err: &InternalError) -> Self {
        match (err.class(), err.origin()) {
            (InternalErrorClass::Infra, InternalErrorOrigin::Infra) => Self::ManagementCall,
            (InternalErrorClass::Domain, InternalErrorOrigin::Domain)
            | (InternalErrorClass::Access, _) => Self::PolicyDenied,
            (InternalErrorClass::Domain, InternalErrorOrigin::Config)
            | (
                InternalErrorClass::Invariant
                | InternalErrorClass::Ops
                | InternalErrorClass::Workflow,
                _,
            ) => Self::InvalidState,
            _ => Self::Unknown,
        }
    }
}

///
/// CanisterOpsMetricKey
///

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct CanisterOpsMetricKey {
    pub operation: CanisterOpsMetricOperation,
    pub role: String,
    pub outcome: CanisterOpsMetricOutcome,
    pub reason: CanisterOpsMetricReason,
}

///
/// CanisterOpsMetrics
///

pub struct CanisterOpsMetrics;

impl CanisterOpsMetrics {
    /// Record one canister operation event for a concrete role label.
    pub fn record(
        operation: CanisterOpsMetricOperation,
        role: &CanisterRole,
        outcome: CanisterOpsMetricOutcome,
        reason: CanisterOpsMetricReason,
    ) {
        Self::record_role_label(operation, role.as_str(), outcome, reason);
    }

    /// Record one canister operation event when no role is available.
    pub fn record_unscoped(
        operation: CanisterOpsMetricOperation,
        outcome: CanisterOpsMetricOutcome,
        reason: CanisterOpsMetricReason,
    ) {
        Self::record_role_label(operation, UNSCOPED_ROLE_LABEL, outcome, reason);
    }

    /// Record one canister operation event when role lookup failed.
    pub fn record_unknown_role(
        operation: CanisterOpsMetricOperation,
        outcome: CanisterOpsMetricOutcome,
        reason: CanisterOpsMetricReason,
    ) {
        Self::record_role_label(operation, UNKNOWN_ROLE_LABEL, outcome, reason);
    }

    // Increment one canister operation counter with a bounded role label.
    fn record_role_label(
        operation: CanisterOpsMetricOperation,
        role: &str,
        outcome: CanisterOpsMetricOutcome,
        reason: CanisterOpsMetricReason,
    ) {
        CANISTER_OPS_METRICS.with_borrow_mut(|counts| {
            let key = CanisterOpsMetricKey {
                operation,
                role: role.to_string(),
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current canister operation metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(CanisterOpsMetricKey, u64)> {
        CANISTER_OPS_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all canister operation metrics.
    #[cfg(test)]
    pub fn reset() {
        CANISTER_OPS_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<CanisterOpsMetricKey, u64> {
        CanisterOpsMetrics::snapshot().into_iter().collect()
    }

    // Verify canister operation counters accumulate by operation, role, outcome, and reason.
    #[test]
    fn canister_ops_metrics_accumulate_by_operation_role_outcome_and_reason() {
        CanisterOpsMetrics::reset();
        let role = CanisterRole::new("app");

        CanisterOpsMetrics::record(
            CanisterOpsMetricOperation::Create,
            &role,
            CanisterOpsMetricOutcome::Started,
            CanisterOpsMetricReason::Ok,
        );
        CanisterOpsMetrics::record(
            CanisterOpsMetricOperation::Create,
            &role,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::Topology,
        );
        CanisterOpsMetrics::record(
            CanisterOpsMetricOperation::Create,
            &role,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::Topology,
        );
        CanisterOpsMetrics::record_unscoped(
            CanisterOpsMetricOperation::Snapshot,
            CanisterOpsMetricOutcome::Completed,
            CanisterOpsMetricReason::Ok,
        );
        CanisterOpsMetrics::record_unscoped(
            CanisterOpsMetricOperation::Restore,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::ManagementCall,
        );

        let map = snapshot_map();

        assert_eq!(
            map.get(&CanisterOpsMetricKey {
                operation: CanisterOpsMetricOperation::Create,
                role: "app".to_string(),
                outcome: CanisterOpsMetricOutcome::Started,
                reason: CanisterOpsMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&CanisterOpsMetricKey {
                operation: CanisterOpsMetricOperation::Create,
                role: "app".to_string(),
                outcome: CanisterOpsMetricOutcome::Failed,
                reason: CanisterOpsMetricReason::Topology,
            }),
            Some(&2)
        );
        assert_eq!(
            map.get(&CanisterOpsMetricKey {
                operation: CanisterOpsMetricOperation::Snapshot,
                role: "unscoped".to_string(),
                outcome: CanisterOpsMetricOutcome::Completed,
                reason: CanisterOpsMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&CanisterOpsMetricKey {
                operation: CanisterOpsMetricOperation::Restore,
                role: "unscoped".to_string(),
                outcome: CanisterOpsMetricOutcome::Failed,
                reason: CanisterOpsMetricReason::ManagementCall,
            }),
            Some(&1)
        );
    }
}
