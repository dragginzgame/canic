mod propagation;

use crate::{
    InternalError,
    api::runtime::install::ModuleSourceRuntimeApi,
    domain::policy::{
        topology::{TopologyPolicy, TopologyPolicyError},
        upgrade::plan_upgrade,
    },
    ops::{
        ic::mgmt::{CanisterInstallMode, MgmtOps},
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
        assert_registered_parent(parent)?;

        let pid = ProvisionWorkflow::create_and_install_canister(&role, parent, extra_arg).await?;

        assert_registered_immediate_parent(pid, parent)?;

        PropagationWorkflow::propagate_topology(pid).await?;
        PropagationWorkflow::propagate_state(pid, &role).await?;

        Ok(CanisterLifecycleResult::created(pid))
    }

    // ───────────────────────── Upgrade ──────────────────────────

    async fn apply_upgrade(pid: Principal) -> Result<CanisterLifecycleResult, InternalError> {
        let registry_data = SubnetRegistryOps::data();
        let registry_input = RegistryPolicyInputMapper::record_to_policy_input(registry_data);

        let record = SubnetRegistryOps::get(pid)
            .ok_or_else(|| InternalError::from(TopologyPolicyError::RegistryEntryMissing(pid)))?;

        let module_source = ModuleSourceRuntimeApi::approved_module_source(&record.role).await?;
        let target_hash = module_source.module_hash().to_vec();

        let status = MgmtOps::canister_status(pid).await?;
        let plan = plan_upgrade(status.module_hash, target_hash.clone());

        if let Some(parent_pid) = record.parent_pid {
            TopologyPolicy::assert_parent_exists(&registry_input, parent_pid)?;
            TopologyPolicy::assert_immediate_parent(&registry_input, pid, parent_pid)?;
        }

        if !plan.should_upgrade {
            log!(
                Topic::CanisterLifecycle,
                Info,
                "canister_upgrade: {pid} already running target module"
            );

            SubnetRegistryOps::update_module_hash(pid, target_hash.clone());

            let registry_data = SubnetRegistryOps::data();
            let registry_input = RegistryPolicyInputMapper::record_to_policy_input(registry_data);
            TopologyPolicy::assert_module_hash(&registry_input, pid, &target_hash)?;

            return Ok(CanisterLifecycleResult::default());
        }

        ModuleInstallWorkflow::install_code(
            CanisterInstallMode::Upgrade(None),
            pid,
            &module_source,
            (),
        )
        .await?;
        SubnetRegistryOps::update_module_hash(pid, target_hash.clone());

        let registry_data = SubnetRegistryOps::data();
        let registry_input = RegistryPolicyInputMapper::record_to_policy_input(registry_data);
        TopologyPolicy::assert_module_hash(&registry_input, pid, &target_hash)?;

        Ok(CanisterLifecycleResult::default())
    }
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
