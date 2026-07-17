//! Module: workflow::canister_lifecycle
//!
//! Responsibility: orchestrate canister create and upgrade lifecycle events.
//! Does not own: endpoint authorization, stable registry schemas, or pure upgrade policy.
//! Boundary: workflow layer coordinating replay, cost guards, IC ops, registry ops, and cascades.

mod propagation;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::{Principal, TC},
    domain::metrics::{
        CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
    },
    domain::policy::pure::{
        topology::{TopologyPolicy, TopologyPolicyError},
        upgrade::plan_upgrade,
    },
    ids::CanisterRole,
    log,
    log::Topic,
    model::replay::{CommandKind, ExternalEffectDescriptor, RecoveryReason},
    ops::{
        cost_guard::{CostGuardOps, CostGuardPermit, CostGuardRequest},
        ic::{
            IcOps,
            mgmt::{CanisterInstallMode, MgmtOps},
        },
        replay::{self as replay_ops, guard::ReplayPending},
        runtime::install_source::{ApprovedModuleSource, ModuleSourceRuntimeApi},
        runtime::metrics::canister_ops::CanisterOpsMetrics,
        runtime::metrics::provisioning::{
            ProvisioningMetricOperation, ProvisioningMetricOutcome, ProvisioningMetricReason,
            ProvisioningMetrics,
        },
        storage::registry::subnet::SubnetRegistryOps,
        topology::input::mapper::TopologyRegistryMapper,
    },
    replay_policy::CostClass,
    workflow::{
        canister_lifecycle::propagation::PropagationWorkflow,
        cost_guard::map_cost_guard_reserve_error, ic::provision::ProvisionWorkflow,
        runtime::install::ModuleInstallWorkflow,
    },
};

///
/// CanisterLifecycleEvent
///
pub enum CanisterLifecycleEvent<'a> {
    Create {
        deployment_permit: &'a CostGuardPermit,
        role: CanisterRole,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    },
    Upgrade {
        cost_context: CanisterUpgradeCostContext,
        pid: Principal,
        replay_pending: &'a ReplayPending,
    },
}

///
/// CanisterUpgradeCostContext
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanisterUpgradeCostContext {
    pub quota_subject: Principal,
    pub payer: Principal,
    pub now_secs: u64,
}

///
/// CanisterLifecycleResult
///
#[derive(Default)]
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
    pub async fn apply(
        event: CanisterLifecycleEvent<'_>,
    ) -> Result<CanisterLifecycleResult, InternalError> {
        match event {
            CanisterLifecycleEvent::Create {
                deployment_permit,
                role,
                parent,
                extra_arg,
            } => Self::apply_create(deployment_permit, role, parent, extra_arg).await,

            CanisterLifecycleEvent::Upgrade {
                cost_context,
                pid,
                replay_pending,
            } => Self::apply_upgrade(cost_context, pid, replay_pending).await,
        }
    }

    // -------------------------------------------------------------------------
    // Creation
    // -------------------------------------------------------------------------

    async fn apply_create(
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

        propagate_topology_with_metrics(pid, &role).await?;
        propagate_state_with_metrics(&role).await?;

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

    // ───────────────────────── Upgrade ──────────────────────────

    async fn apply_upgrade(
        cost_context: CanisterUpgradeCostContext,
        pid: Principal,
        replay_pending: &ReplayPending,
    ) -> Result<CanisterLifecycleResult, InternalError> {
        let (role, parent_pid) = upgrade_target(pid)?;

        record_provisioning(
            &role,
            ProvisioningMetricOperation::Upgrade,
            ProvisioningMetricOutcome::Started,
            ProvisioningMetricReason::Ok,
        );
        record_canister_op(
            &role,
            CanisterOpsMetricOperation::Upgrade,
            CanisterOpsMetricOutcome::Started,
            CanisterOpsMetricReason::Ok,
        );

        let module_source = upgrade_module_source(&role).await?;
        let target_hash = module_source.module_hash().to_vec();
        let current_hash = upgrade_current_hash(pid, &role).await?;
        let plan = plan_upgrade(current_hash, target_hash.clone());

        assert_upgrade_parent(pid, parent_pid, &role)?;

        if !plan.should_upgrade {
            log!(
                Topic::CanisterLifecycle,
                Info,
                "canister_upgrade: {pid} already running target module"
            );

            SubnetRegistryOps::update_module_hash(pid, target_hash.clone());
            assert_upgrade_module_hash(pid, &target_hash, &role)?;
            record_canister_op(
                &role,
                CanisterOpsMetricOperation::Upgrade,
                CanisterOpsMetricOutcome::Skipped,
                CanisterOpsMetricReason::AlreadyExists,
            );
            record_provisioning(
                &role,
                ProvisioningMetricOperation::Upgrade,
                ProvisioningMetricOutcome::Skipped,
                ProvisioningMetricReason::AlreadyCurrent,
            );

            return Ok(CanisterLifecycleResult::default());
        }

        let cost_permit = match reserve_canister_upgrade_cost_guard(
            cost_context,
            IcOps::canister_cycle_balance().to_u128(),
        ) {
            Ok(permit) => permit,
            Err(err) => {
                record_canister_op_failure(&role, CanisterOpsMetricOperation::Upgrade, &err);
                record_provisioning_failure(&role, ProvisioningMetricOperation::Upgrade, &err);
                return Err(err);
            }
        };
        log!(
            Topic::CanisterLifecycle,
            Info,
            "canister_upgrade: deployment cost guard reserved command_kind={} quota_subject={} payer={} target={}",
            CANISTER_UPGRADE_COMMAND_KIND,
            cost_context.quota_subject,
            cost_context.payer,
            pid
        );

        if let Err(err) = execute_costed_upgrade(
            &role,
            pid,
            &target_hash,
            &module_source,
            &cost_permit,
            replay_pending,
        )
        .await
        {
            record_canister_op_failure(&role, CanisterOpsMetricOperation::Upgrade, &err);
            record_provisioning_failure(&role, ProvisioningMetricOperation::Upgrade, &err);
            return Err(err);
        }
        record_canister_op(
            &role,
            CanisterOpsMetricOperation::Upgrade,
            CanisterOpsMetricOutcome::Completed,
            CanisterOpsMetricReason::Ok,
        );
        record_provisioning(
            &role,
            ProvisioningMetricOperation::Upgrade,
            ProvisioningMetricOutcome::Completed,
            ProvisioningMetricReason::Ok,
        );

        Ok(CanisterLifecycleResult::default())
    }
}

async fn execute_costed_upgrade(
    role: &CanisterRole,
    pid: Principal,
    target_hash: &[u8],
    module_source: &ApprovedModuleSource,
    cost_permit: &CostGuardPermit,
    replay_pending: &ReplayPending,
) -> Result<(), InternalError> {
    if let Err(replay_err) = replay_ops::mark_root_replay_costed_external_effect(
        replay_pending,
        ExternalEffectDescriptor::ManagementCall {
            canister: pid,
            method: "install_code:upgrade".to_string(),
        },
        cost_permit,
        crate::ops::replay::guard::secs_to_ns(IcOps::now_secs()),
    ) {
        return Err(CostGuardOps::recover_after_failure(
            cost_permit,
            IcOps::now_secs(),
            map_upgrade_replay_store_error(replay_err),
        ));
    }

    if let Err(err) = ModuleInstallWorkflow::install_code_with_permit(
        cost_permit,
        CanisterInstallMode::Upgrade(None),
        pid,
        module_source,
        (),
    )
    .await
    {
        let err = CostGuardOps::recover_after_failure(cost_permit, IcOps::now_secs(), err);
        return match replay_ops::mark_root_replay_recovery_required(
            replay_pending,
            RecoveryReason::ExternalEffectStatusUnknown,
            crate::ops::replay::guard::secs_to_ns(IcOps::now_secs()),
        )
        .map_err(map_upgrade_replay_store_error)
        {
            Ok(()) => Err(err),
            Err(recovery_err) => Err(err.with_diagnostic_context(format!(
                "root upgrade replay recovery marker failed: {recovery_err}"
            ))),
        };
    }

    SubnetRegistryOps::update_module_hash(pid, target_hash.to_vec());
    if let Err(mut err) = assert_upgrade_module_hash(pid, target_hash, role) {
        if let Err(settlement_err) = CostGuardOps::complete(cost_permit, IcOps::now_secs()) {
            err = err.with_diagnostic_context(format!(
                "root upgrade cost settlement also failed: {settlement_err}"
            ));
        }
        if let Err(recovery_err) = replay_ops::mark_root_replay_recovery_required(
            replay_pending,
            RecoveryReason::StateProjectionFailed,
            crate::ops::replay::guard::secs_to_ns(IcOps::now_secs()),
        )
        .map_err(map_upgrade_replay_store_error)
        {
            err = err.with_diagnostic_context(format!(
                "root upgrade replay recovery marker failed: {recovery_err}"
            ));
        }
        return Err(err);
    }
    if let Err(err) = CostGuardOps::complete(cost_permit, IcOps::now_secs()) {
        return match replay_ops::mark_root_replay_recovery_required(
            replay_pending,
            RecoveryReason::CostSettlementFailed,
            crate::ops::replay::guard::secs_to_ns(IcOps::now_secs()),
        )
        .map_err(map_upgrade_replay_store_error)
        {
            Ok(()) => Err(err),
            Err(recovery_err) => Err(err.with_diagnostic_context(format!(
                "root upgrade replay recovery marker failed: {recovery_err}"
            ))),
        };
    }
    Ok(())
}

fn map_upgrade_replay_store_error(
    err: crate::ops::replay::receipt::ReplayReceiptStoreError,
) -> InternalError {
    InternalError::workflow(
        InternalErrorOrigin::Workflow,
        format!("root upgrade replay receipt update failed: {err}"),
    )
}

const CANISTER_UPGRADE_COMMAND_KIND: &str = "management.canister_upgrade.v1";
const CANISTER_UPGRADE_DEPLOYMENT_QUOTA_WINDOW_SECONDS: u64 = 60;
const MAX_CANISTER_UPGRADE_DEPLOYMENT_OPERATIONS_PER_WINDOW: u64 = 10;
const CANISTER_UPGRADE_CYCLE_RESERVATION_CYCLES: u128 = 1_000_000_000;
const MIN_CANISTER_UPGRADE_CYCLES_AFTER_RESERVATION: u128 = TC;

fn reserve_canister_upgrade_cost_guard(
    cost_context: CanisterUpgradeCostContext,
    current_cycle_balance: u128,
) -> Result<CostGuardPermit, InternalError> {
    CostGuardOps::reserve(canister_upgrade_cost_guard_request(
        cost_context,
        current_cycle_balance,
    ))
    .map_err(map_cost_guard_reserve_error)
}

pub(super) fn canister_upgrade_cost_guard_request(
    cost_context: CanisterUpgradeCostContext,
    current_cycle_balance: u128,
) -> CostGuardRequest {
    CostGuardRequest {
        cost_class: CostClass::ManagementDeployment,
        command_kind: CommandKind::new(CANISTER_UPGRADE_COMMAND_KIND)
            .expect("canister upgrade command kind is a valid static label"),
        quota_subject: cost_context.quota_subject,
        payer: cost_context.payer,
        now_secs: cost_context.now_secs,
        quota_window_secs: CANISTER_UPGRADE_DEPLOYMENT_QUOTA_WINDOW_SECONDS,
        max_operations_per_window: MAX_CANISTER_UPGRADE_DEPLOYMENT_OPERATIONS_PER_WINDOW,
        current_cycle_balance,
        cycle_reservation_cycles: CANISTER_UPGRADE_CYCLE_RESERVATION_CYCLES,
        min_cycles_after_reservation: MIN_CANISTER_UPGRADE_CYCLES_AFTER_RESERVATION,
    }
}

// Resolve the registry role and parent for one upgrade target.
fn upgrade_target(pid: Principal) -> Result<(CanisterRole, Option<Principal>), InternalError> {
    let Some(target) = SubnetRegistryOps::role_parent(pid) else {
        CanisterOpsMetrics::record_unknown_role(
            CanisterOpsMetricOperation::Upgrade,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::NotFound,
        );
        ProvisioningMetrics::record_unknown_role(
            ProvisioningMetricOperation::Upgrade,
            ProvisioningMetricOutcome::Failed,
            ProvisioningMetricReason::NotFound,
        );
        return Err(InternalError::from(
            TopologyPolicyError::RegistryEntryMissing(pid),
        ));
    };

    Ok(target)
}

// Resolve the approved module source for one upgrade target role.
async fn upgrade_module_source(role: &CanisterRole) -> Result<ApprovedModuleSource, InternalError> {
    match ModuleSourceRuntimeApi::approved_module_source(role).await {
        Ok(module_source) => Ok(module_source),
        Err(err) => {
            record_canister_op(
                role,
                CanisterOpsMetricOperation::Upgrade,
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::MissingWasm,
            );
            record_provisioning(
                role,
                ProvisioningMetricOperation::Upgrade,
                ProvisioningMetricOutcome::Failed,
                ProvisioningMetricReason::MissingWasm,
            );
            Err(err)
        }
    }
}

// Read the currently installed module hash for one upgrade target.
async fn upgrade_current_hash(
    pid: Principal,
    role: &CanisterRole,
) -> Result<Option<Vec<u8>>, InternalError> {
    match MgmtOps::canister_status(pid).await {
        Ok(status) => Ok(status.module_hash),
        Err(err) => {
            record_canister_op_failure(role, CanisterOpsMetricOperation::Upgrade, &err);
            record_provisioning_failure(role, ProvisioningMetricOperation::Upgrade, &err);
            Err(err)
        }
    }
}

// Assert the upgrade target is still attached to its recorded parent.
fn assert_upgrade_parent(
    pid: Principal,
    parent_pid: Option<Principal>,
    role: &CanisterRole,
) -> Result<(), InternalError> {
    let Some(parent_pid) = parent_pid else {
        return Ok(());
    };

    let registry_data = SubnetRegistryOps::data();
    let registry_input = TopologyRegistryMapper::data_to_registry(registry_data);

    if let Err(err) = TopologyPolicy::assert_parent_exists(&registry_input, parent_pid) {
        record_canister_op(
            role,
            CanisterOpsMetricOperation::Upgrade,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::Topology,
        );
        record_provisioning(
            role,
            ProvisioningMetricOperation::Upgrade,
            ProvisioningMetricOutcome::Failed,
            ProvisioningMetricReason::Topology,
        );
        return Err(err);
    }

    if let Err(err) = TopologyPolicy::assert_immediate_parent(&registry_input, pid, parent_pid) {
        record_canister_op(
            role,
            CanisterOpsMetricOperation::Upgrade,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::Topology,
        );
        record_provisioning(
            role,
            ProvisioningMetricOperation::Upgrade,
            ProvisioningMetricOutcome::Failed,
            ProvisioningMetricReason::Topology,
        );
        return Err(err);
    }

    Ok(())
}

// Assert the registry reflects the target module hash after upgrade bookkeeping.
fn assert_upgrade_module_hash(
    pid: Principal,
    target_hash: &[u8],
    role: &CanisterRole,
) -> Result<(), InternalError> {
    let registry_data = SubnetRegistryOps::data();
    let registry_input = TopologyRegistryMapper::data_to_registry(registry_data);

    if let Err(err) = TopologyPolicy::assert_module_hash(&registry_input, pid, target_hash) {
        record_canister_op(
            role,
            CanisterOpsMetricOperation::Upgrade,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::Topology,
        );
        record_provisioning(
            role,
            ProvisioningMetricOperation::Upgrade,
            ProvisioningMetricOutcome::Failed,
            ProvisioningMetricReason::Topology,
        );
        return Err(err);
    }

    Ok(())
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
) -> Result<(), InternalError> {
    record_provisioning(
        role,
        ProvisioningMetricOperation::PropagateTopology,
        ProvisioningMetricOutcome::Started,
        ProvisioningMetricReason::Ok,
    );
    if let Err(err) = PropagationWorkflow::propagate_topology(pid).await {
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
    record_provisioning(
        role,
        ProvisioningMetricOperation::PropagateTopology,
        ProvisioningMetricOutcome::Completed,
        ProvisioningMetricReason::Ok,
    );
    Ok(())
}

// Propagate state and record workflow-level provisioning outcomes.
async fn propagate_state_with_metrics(role: &CanisterRole) -> Result<(), InternalError> {
    record_provisioning(
        role,
        ProvisioningMetricOperation::PropagateState,
        ProvisioningMetricOutcome::Started,
        ProvisioningMetricReason::Ok,
    );
    if let Err(err) = PropagationWorkflow::propagate_state(role).await {
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
    record_provisioning(
        role,
        ProvisioningMetricOperation::PropagateState,
        ProvisioningMetricOutcome::Completed,
        ProvisioningMetricReason::Ok,
    );
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn canister_upgrade_cost_guard_request_uses_deployment_policy() {
        let cost_context = CanisterUpgradeCostContext {
            quota_subject: p(7),
            payer: p(8),
            now_secs: 9_000,
        };

        let request = canister_upgrade_cost_guard_request(cost_context, 100 * TC);

        assert_eq!(request.cost_class, CostClass::ManagementDeployment);
        assert_eq!(
            request.command_kind.as_str(),
            "management.canister_upgrade.v1"
        );
        assert_eq!(request.quota_subject, cost_context.quota_subject);
        assert_eq!(request.payer, cost_context.payer);
        assert_eq!(request.now_secs, cost_context.now_secs);
        assert_eq!(request.quota_window_secs, 60);
        assert_eq!(request.max_operations_per_window, 10);
        assert_eq!(request.current_cycle_balance, 100 * TC);
        assert_eq!(request.cycle_reservation_cycles, 1_000_000_000);
        assert_eq!(request.min_cycles_after_reservation, TC);
    }
}
