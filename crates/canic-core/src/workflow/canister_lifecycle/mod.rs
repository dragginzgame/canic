mod propagation;

use crate::{
    InternalError,
    api::runtime::install::{ApprovedModuleSource, ModuleSourceRuntimeApi},
    domain::policy::{
        topology::{TopologyPolicy, TopologyPolicyError},
        upgrade::plan_upgrade,
    },
    ops::{
        ic::mgmt::{CanisterInstallMode, MgmtOps},
        runtime::metrics::canister_ops::{
            CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
            CanisterOpsMetrics,
        },
        storage::registry::subnet::SubnetRegistryOps,
        topology::policy::mapper::RegistryPolicyInputMapper,
    },
    workflow::{
        canister_lifecycle::propagation::PropagationWorkflow, ic::provision::ProvisionWorkflow,
        prelude::*, runtime::install::ModuleInstallWorkflow,
    },
};

///
/// CanisterLifecycleEvent
///
pub enum CanisterLifecycleEvent {
    Create {
        role: CanisterRole,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    },
    Upgrade {
        pid: Principal,
    },
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
        event: CanisterLifecycleEvent,
    ) -> Result<CanisterLifecycleResult, InternalError> {
        match event {
            CanisterLifecycleEvent::Create {
                role,
                parent,
                extra_arg,
            } => Self::apply_create(role, parent, extra_arg).await,

            CanisterLifecycleEvent::Upgrade { pid } => Self::apply_upgrade(pid).await,
        }
    }

    // ───────────────────────── Creation ─────────────────────────

    async fn apply_create(
        role: CanisterRole,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<CanisterLifecycleResult, InternalError> {
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
            return Err(err);
        }

        let pid =
            match ProvisionWorkflow::create_and_install_canister(&role, parent, extra_arg).await {
                Ok(pid) => pid,
                Err(err) => {
                    record_canister_op_failure(&role, CanisterOpsMetricOperation::Create, &err);
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
            return Err(err);
        }

        if let Err(err) = PropagationWorkflow::propagate_topology(pid).await {
            record_canister_op(
                &role,
                CanisterOpsMetricOperation::Create,
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::TopologyPropagation,
            );
            return Err(err);
        }

        if let Err(err) = PropagationWorkflow::propagate_state(pid, &role).await {
            record_canister_op(
                &role,
                CanisterOpsMetricOperation::Create,
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::StatePropagation,
            );
            return Err(err);
        }

        record_canister_op(
            &role,
            CanisterOpsMetricOperation::Create,
            CanisterOpsMetricOutcome::Completed,
            CanisterOpsMetricReason::Ok,
        );

        Ok(CanisterLifecycleResult::created(pid))
    }

    // ───────────────────────── Upgrade ──────────────────────────

    async fn apply_upgrade(pid: Principal) -> Result<CanisterLifecycleResult, InternalError> {
        let (role, parent_pid) = upgrade_target(pid)?;

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

            return Ok(CanisterLifecycleResult::default());
        }

        if let Err(err) = ModuleInstallWorkflow::install_code(
            CanisterInstallMode::Upgrade(None),
            pid,
            &module_source,
            (),
        )
        .await
        {
            record_canister_op_failure(&role, CanisterOpsMetricOperation::Upgrade, &err);
            return Err(err);
        }
        SubnetRegistryOps::update_module_hash(pid, target_hash.clone());
        assert_upgrade_module_hash(pid, &target_hash, &role)?;
        record_canister_op(
            &role,
            CanisterOpsMetricOperation::Upgrade,
            CanisterOpsMetricOutcome::Completed,
            CanisterOpsMetricReason::Ok,
        );

        Ok(CanisterLifecycleResult::default())
    }
}

// Resolve the registry role and parent for one upgrade target.
fn upgrade_target(pid: Principal) -> Result<(CanisterRole, Option<Principal>), InternalError> {
    let Some(record) = SubnetRegistryOps::get(pid) else {
        CanisterOpsMetrics::record_unknown_role(
            CanisterOpsMetricOperation::Upgrade,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::NotFound,
        );
        return Err(InternalError::from(
            TopologyPolicyError::RegistryEntryMissing(pid),
        ));
    };

    Ok((record.role, record.parent_pid))
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
    let registry_input = RegistryPolicyInputMapper::record_to_policy_input(registry_data);

    if let Err(err) = TopologyPolicy::assert_parent_exists(&registry_input, parent_pid) {
        record_canister_op(
            role,
            CanisterOpsMetricOperation::Upgrade,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::Topology,
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
    let registry_input = RegistryPolicyInputMapper::record_to_policy_input(registry_data);

    if let Err(err) = TopologyPolicy::assert_module_hash(&registry_input, pid, target_hash) {
        record_canister_op(
            role,
            CanisterOpsMetricOperation::Upgrade,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::Topology,
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
    let record =
        SubnetRegistryOps::get(pid).ok_or(TopologyPolicyError::RegistryEntryMissing(pid))?;

    if record.parent_pid == Some(expected_parent) {
        Ok(())
    } else {
        Err(TopologyPolicyError::ImmediateParentMismatch {
            pid,
            expected: expected_parent,
            found: record.parent_pid,
        }
        .into())
    }
}
