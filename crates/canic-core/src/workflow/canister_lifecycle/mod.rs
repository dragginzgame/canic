mod propagation;

use crate::{
    InternalError,
    domain::policy::{
        topology::{TopologyPolicy, TopologyPolicyError},
        upgrade::plan_upgrade,
    },
    ops::{
        ic::mgmt::MgmtOps, runtime::wasm::WasmOps, storage::registry::subnet::SubnetRegistryOps,
        topology::policy::mapper::RegistryPolicyInputMapper,
    },
    workflow::{
        canister_lifecycle::propagation::PropagationWorkflow, ic::provision::ProvisionWorkflow,
        prelude::*,
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
    pub(crate) async fn apply(
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
        let registry_data = SubnetRegistryOps::data();
        let registry_input = RegistryPolicyInputMapper::record_to_policy_input(registry_data);
        TopologyPolicy::assert_parent_exists(&registry_input, parent)?;

        let pid = ProvisionWorkflow::create_and_install_canister(&role, parent, extra_arg).await?;

        let registry_data = SubnetRegistryOps::data();
        let registry_input = RegistryPolicyInputMapper::record_to_policy_input(registry_data);
        TopologyPolicy::assert_immediate_parent(&registry_input, pid, parent)?;

        PropagationWorkflow::propagate_topology(pid).await?;
        PropagationWorkflow::propagate_state(&role).await?;

        Ok(CanisterLifecycleResult::created(pid))
    }

    // ───────────────────────── Upgrade ──────────────────────────

    async fn apply_upgrade(pid: Principal) -> Result<CanisterLifecycleResult, InternalError> {
        let registry_data = SubnetRegistryOps::data();
        let registry_input = RegistryPolicyInputMapper::record_to_policy_input(registry_data);

        let record = SubnetRegistryOps::get(pid)
            .ok_or_else(|| InternalError::from(TopologyPolicyError::RegistryEntryMissing(pid)))?;

        let wasm = WasmOps::try_get(&record.role)?;
        let target_hash = wasm.module_hash();

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

        MgmtOps::upgrade_canister(pid, wasm.bytes()).await?;
        SubnetRegistryOps::update_module_hash(pid, target_hash.clone());

        let registry_data = SubnetRegistryOps::data();
        let registry_input = RegistryPolicyInputMapper::record_to_policy_input(registry_data);
        TopologyPolicy::assert_module_hash(&registry_input, pid, &target_hash)?;

        Ok(CanisterLifecycleResult::default())
    }
}
