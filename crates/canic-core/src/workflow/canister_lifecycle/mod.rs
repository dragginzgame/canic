//! Module: workflow::canister_lifecycle
//!
//! Responsibility: orchestrate the retained generic canister creation lifecycle.
//! Does not own: endpoint authorization, stable registry schemas, or Component lifecycle journals.
//! Boundary: workflow layer coordinating generic creation, registry ops, and cascades.

mod propagation;

use crate::{
    InternalError,
    cdk::types::Principal,
    domain::metrics::{
        CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
    },
    domain::policy::pure::topology::TopologyPolicyError,
    dto::cascade::{StateSnapshotInput, TopologySnapshotInput},
    ids::CanisterRole,
    ops::{
        cost_guard::CostGuardPermit,
        runtime::metrics::canister_ops::CanisterOpsMetrics,
        runtime::metrics::provisioning::{
            ProvisioningMetricOperation, ProvisioningMetricOutcome, ProvisioningMetricReason,
            ProvisioningMetrics,
        },
        storage::registry::subnet::SubnetRegistryOps,
    },
    workflow::{
        canister_lifecycle::propagation::PropagationWorkflow, ic::provision::ProvisionWorkflow,
        runtime::fleet_activation::FleetActivationWorkflow,
    },
};

///
/// CanisterLifecycleResult
///
pub struct CanisterLifecycleResult {
    pub new_canister_pid: Option<Principal>,
}

impl CanisterLifecycleResult {
    #[must_use]
    pub const fn created(pid: Principal) -> Self {
        Self {
            new_canister_pid: Some(pid),
        }
    }
}

///
/// CanisterLifecycleWorkflow
///
pub struct CanisterLifecycleWorkflow;

impl CanisterLifecycleWorkflow {
    pub async fn create(
        deployment_permit: &CostGuardPermit,
        role: CanisterRole,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<CanisterLifecycleResult, InternalError> {
        record_provisioning(
            &role,
            ProvisioningMetricOperation::Create,
            ProvisioningMetricOutcome::Started,
            ProvisioningMetricReason::Ok,
        );
        record_canister_op(
            &role,
            CanisterOpsMetricOperation::Create,
            CanisterOpsMetricOutcome::Started,
            CanisterOpsMetricReason::Ok,
        );

        if let Err(err) = assert_registered_parent(parent) {
            record_canister_op(
                &role,
                CanisterOpsMetricOperation::Create,
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::Topology,
            );
            record_provisioning(
                &role,
                ProvisioningMetricOperation::Create,
                ProvisioningMetricOutcome::Failed,
                ProvisioningMetricReason::Topology,
            );
            return Err(err);
        }

        let pid = match ProvisionWorkflow::create_and_install_canister(
            deployment_permit,
            &role,
            parent,
            extra_arg,
        )
        .await
        {
            Ok(pid) => pid,
            Err(err) => {
                record_canister_op_failure(&role, CanisterOpsMetricOperation::Create, &err);
                record_provisioning_failure(&role, ProvisioningMetricOperation::Create, &err);
                return Err(err);
            }
        };

        if let Err(err) = assert_registered_immediate_parent(pid, parent) {
            record_canister_op(
                &role,
                CanisterOpsMetricOperation::Create,
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::Topology,
            );
            record_provisioning(
                &role,
                ProvisioningMetricOperation::Create,
                ProvisioningMetricOutcome::Failed,
                ProvisioningMetricReason::Topology,
            );
            return Err(err);
        }

        let topology = propagate_topology_with_metrics(pid, &role).await?;
        let state = propagate_state_with_metrics(&role).await?;
        if let Err(err) =
            FleetActivationWorkflow::complete_provisioned_nonroot_activation(pid, state, topology)
                .await
        {
            record_canister_op_failure(&role, CanisterOpsMetricOperation::Create, &err);
            record_provisioning_failure(&role, ProvisioningMetricOperation::Create, &err);
            return Err(err);
        }

        record_canister_op(
            &role,
            CanisterOpsMetricOperation::Create,
            CanisterOpsMetricOutcome::Completed,
            CanisterOpsMetricReason::Ok,
        );
        record_provisioning(
            &role,
            ProvisioningMetricOperation::Create,
            ProvisioningMetricOutcome::Completed,
            ProvisioningMetricReason::Ok,
        );

        Ok(CanisterLifecycleResult::created(pid))
    }
}

// Record one canister operation metric for a known role.
fn record_canister_op(
    role: &CanisterRole,
    operation: CanisterOpsMetricOperation,
    outcome: CanisterOpsMetricOutcome,
    reason: CanisterOpsMetricReason,
) {
    CanisterOpsMetrics::record(operation, role, outcome, reason);
}

// Record one failed canister operation metric using the structured error category.
fn record_canister_op_failure(
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

// Propagate topology and record workflow-level provisioning outcomes.
async fn propagate_topology_with_metrics(
    pid: Principal,
    role: &CanisterRole,
) -> Result<TopologySnapshotInput, InternalError> {
    record_provisioning(
        role,
        ProvisioningMetricOperation::PropagateTopology,
        ProvisioningMetricOutcome::Started,
        ProvisioningMetricReason::Ok,
    );
    let input = match PropagationWorkflow::propagate_topology(pid).await {
        Ok(input) => input,
        Err(err) => {
            record_canister_op(
                role,
                CanisterOpsMetricOperation::Create,
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::TopologyPropagation,
            );
            record_provisioning(
                role,
                ProvisioningMetricOperation::PropagateTopology,
                ProvisioningMetricOutcome::Failed,
                ProvisioningMetricReason::TopologyPropagation,
            );
            return Err(err);
        }
    };
    record_provisioning(
        role,
        ProvisioningMetricOperation::PropagateTopology,
        ProvisioningMetricOutcome::Completed,
        ProvisioningMetricReason::Ok,
    );
    Ok(input)
}

// Propagate state and record workflow-level provisioning outcomes.
async fn propagate_state_with_metrics(
    role: &CanisterRole,
) -> Result<StateSnapshotInput, InternalError> {
    record_provisioning(
        role,
        ProvisioningMetricOperation::PropagateState,
        ProvisioningMetricOutcome::Started,
        ProvisioningMetricReason::Ok,
    );
    let input = match PropagationWorkflow::propagate_state(role).await {
        Ok(input) => input,
        Err(err) => {
            record_canister_op(
                role,
                CanisterOpsMetricOperation::Create,
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::StatePropagation,
            );
            record_provisioning(
                role,
                ProvisioningMetricOperation::PropagateState,
                ProvisioningMetricOutcome::Failed,
                ProvisioningMetricReason::StatePropagation,
            );
            return Err(err);
        }
    };
    record_provisioning(
        role,
        ProvisioningMetricOperation::PropagateState,
        ProvisioningMetricOutcome::Completed,
        ProvisioningMetricReason::Ok,
    );
    Ok(input)
}

// Record one provisioning metric for a known role.
fn record_provisioning(
    role: &CanisterRole,
    operation: ProvisioningMetricOperation,
    outcome: ProvisioningMetricOutcome,
    reason: ProvisioningMetricReason,
) {
    ProvisioningMetrics::record(operation, role, outcome, reason);
}

// Record one failed provisioning metric using the structured error category.
fn record_provisioning_failure(
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

// Check that the requested parent already exists without exporting the full registry.
fn assert_registered_parent(parent_pid: Principal) -> Result<(), InternalError> {
    if SubnetRegistryOps::is_registered(parent_pid) {
        Ok(())
    } else {
        Err(TopologyPolicyError::ParentNotFound(parent_pid).into())
    }
}

// Check that the created child is attached to the expected direct parent.
fn assert_registered_immediate_parent(
    pid: Principal,
    expected_parent: Principal,
) -> Result<(), InternalError> {
    let (_, parent_pid) = SubnetRegistryOps::role_parent(pid)
        .ok_or(TopologyPolicyError::RegistryEntryMissing(pid))?;

    if parent_pid == Some(expected_parent) {
        Ok(())
    } else {
        Err(TopologyPolicyError::ImmediateParentMismatch {
            pid,
            expected: expected_parent,
            found: parent_pid,
        }
        .into())
    }
}
