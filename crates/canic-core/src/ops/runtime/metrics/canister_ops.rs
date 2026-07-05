//! Module: ops::runtime::metrics::canister_ops
//!
//! Responsibility: record and snapshot low-cardinality runtime metrics for the canister_ops family.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use crate::{InternalError, InternalErrorClass, InternalErrorOrigin, ids::CanisterRole};
use std::{cell::RefCell, collections::HashMap};

pub use crate::domain::metrics::{
    CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
};

const UNSCOPED_ROLE_LABEL: &str = "unscoped";
const UNKNOWN_ROLE_LABEL: &str = "unknown";

thread_local! {
    static CANISTER_OPS_METRICS: RefCell<HashMap<CanisterOpsMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

impl CanisterOpsMetricReason {
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
/// Composite key for one low-cardinality canister operation counter.
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
/// Operations-layer recorder for canister operation counters.
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

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
