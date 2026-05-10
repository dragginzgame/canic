use crate::{InternalError, InternalErrorClass, InternalErrorOrigin, ids::CanisterRole};
use std::{cell::RefCell, collections::HashMap};

const UNKNOWN_ROLE_LABEL: &str = "unknown";

thread_local! {
    static PROVISIONING_METRICS: RefCell<HashMap<ProvisioningMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// ProvisioningMetricOperation
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ProvisioningMetricOperation {
    Allocate,
    Create,
    Install,
    PropagateState,
    PropagateTopology,
    ResolveModule,
    Upgrade,
}

///
/// ProvisioningMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ProvisioningMetricOutcome {
    Completed,
    Failed,
    Skipped,
    Started,
}

///
/// ProvisioningMetricReason
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum ProvisioningMetricReason {
    AlreadyCurrent,
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

impl ProvisioningMetricReason {
    /// Classify one internal error into a bounded provisioning metric reason.
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
/// ProvisioningMetricKey
///

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ProvisioningMetricKey {
    pub operation: ProvisioningMetricOperation,
    pub role: String,
    pub outcome: ProvisioningMetricOutcome,
    pub reason: ProvisioningMetricReason,
}

///
/// ProvisioningMetrics
///

pub struct ProvisioningMetrics;

impl ProvisioningMetrics {
    /// Record one provisioning workflow event for a concrete role label.
    pub fn record(
        operation: ProvisioningMetricOperation,
        role: &CanisterRole,
        outcome: ProvisioningMetricOutcome,
        reason: ProvisioningMetricReason,
    ) {
        Self::record_role_label(operation, role.as_str(), outcome, reason);
    }

    /// Record one provisioning workflow event when role lookup failed.
    pub fn record_unknown_role(
        operation: ProvisioningMetricOperation,
        outcome: ProvisioningMetricOutcome,
        reason: ProvisioningMetricReason,
    ) {
        Self::record_role_label(operation, UNKNOWN_ROLE_LABEL, outcome, reason);
    }

    // Increment one provisioning counter with a bounded role label.
    fn record_role_label(
        operation: ProvisioningMetricOperation,
        role: &str,
        outcome: ProvisioningMetricOutcome,
        reason: ProvisioningMetricReason,
    ) {
        PROVISIONING_METRICS.with_borrow_mut(|counts| {
            let key = ProvisioningMetricKey {
                operation,
                role: role.to_string(),
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current provisioning metric table as stable rows.
    #[must_use]
    #[cfg(test)]
    pub fn snapshot() -> Vec<(ProvisioningMetricKey, u64)> {
        PROVISIONING_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all provisioning metrics.
    #[cfg(test)]
    pub fn reset() {
        PROVISIONING_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<ProvisioningMetricKey, u64> {
        ProvisioningMetrics::snapshot().into_iter().collect()
    }

    // Verify provisioning counters accumulate by operation, role, outcome, and reason.
    #[test]
    fn provisioning_metrics_accumulate_by_operation_role_outcome_and_reason() {
        ProvisioningMetrics::reset();
        let role = CanisterRole::new("app");

        ProvisioningMetrics::record(
            ProvisioningMetricOperation::Install,
            &role,
            ProvisioningMetricOutcome::Started,
            ProvisioningMetricReason::Ok,
        );
        ProvisioningMetrics::record(
            ProvisioningMetricOperation::Install,
            &role,
            ProvisioningMetricOutcome::Failed,
            ProvisioningMetricReason::MissingWasm,
        );
        ProvisioningMetrics::record(
            ProvisioningMetricOperation::Install,
            &role,
            ProvisioningMetricOutcome::Failed,
            ProvisioningMetricReason::MissingWasm,
        );
        ProvisioningMetrics::record_unknown_role(
            ProvisioningMetricOperation::Upgrade,
            ProvisioningMetricOutcome::Failed,
            ProvisioningMetricReason::NotFound,
        );

        let map = snapshot_map();
        assert_eq!(
            map.get(&ProvisioningMetricKey {
                operation: ProvisioningMetricOperation::Install,
                role: "app".to_string(),
                outcome: ProvisioningMetricOutcome::Started,
                reason: ProvisioningMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&ProvisioningMetricKey {
                operation: ProvisioningMetricOperation::Install,
                role: "app".to_string(),
                outcome: ProvisioningMetricOutcome::Failed,
                reason: ProvisioningMetricReason::MissingWasm,
            }),
            Some(&2)
        );
        assert_eq!(
            map.get(&ProvisioningMetricKey {
                operation: ProvisioningMetricOperation::Upgrade,
                role: UNKNOWN_ROLE_LABEL.to_string(),
                outcome: ProvisioningMetricOutcome::Failed,
                reason: ProvisioningMetricReason::NotFound,
            }),
            Some(&1)
        );
    }
}
