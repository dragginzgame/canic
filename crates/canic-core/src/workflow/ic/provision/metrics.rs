use crate::{
    InternalError,
    ops::runtime::metrics::{
        canister_ops::{
            CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
            CanisterOpsMetrics,
        },
        provisioning::{
            ProvisioningMetricOperation, ProvisioningMetricOutcome, ProvisioningMetricReason,
            ProvisioningMetrics,
        },
    },
    workflow::prelude::*,
};

// Record one canister operation metric for a known role.
pub(super) fn record_canister_op(
    role: &CanisterRole,
    operation: CanisterOpsMetricOperation,
    outcome: CanisterOpsMetricOutcome,
    reason: CanisterOpsMetricReason,
) {
    CanisterOpsMetrics::record(operation, role, outcome, reason);
}

// Record one failed canister operation metric using the structured error category.
pub(super) fn record_canister_op_failure(
    role: &CanisterRole,
    operation: CanisterOpsMetricOperation,
    err: &InternalError,
) {
    record_canister_op(
        role,
        operation,
        CanisterOpsMetricOutcome::Failed,
        CanisterOpsMetricReason::from_error(err),
    );
}

// Record one provisioning metric for a known role.
pub(super) fn record_provisioning(
    role: &CanisterRole,
    operation: ProvisioningMetricOperation,
    outcome: ProvisioningMetricOutcome,
    reason: ProvisioningMetricReason,
) {
    ProvisioningMetrics::record(operation, role, outcome, reason);
}

// Record one failed provisioning metric using the structured error category.
pub(super) fn record_provisioning_failure(
    role: &CanisterRole,
    operation: ProvisioningMetricOperation,
    err: &InternalError,
) {
    record_provisioning(
        role,
        operation,
        ProvisioningMetricOutcome::Failed,
        ProvisioningMetricReason::from_error(err),
    );
}

// Record one delete metric using the registry role when it is still available.
pub(super) fn record_delete_metric(
    role: Option<&CanisterRole>,
    outcome: CanisterOpsMetricOutcome,
    reason: CanisterOpsMetricReason,
) {
    if let Some(role) = role {
        CanisterOpsMetrics::record(CanisterOpsMetricOperation::Delete, role, outcome, reason);
    } else {
        CanisterOpsMetrics::record_unknown_role(
            CanisterOpsMetricOperation::Delete,
            outcome,
            reason,
        );
    }
}
