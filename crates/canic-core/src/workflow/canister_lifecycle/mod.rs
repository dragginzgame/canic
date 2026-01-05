pub mod propagation;

use crate::{
    Error,
    domain::policy::{topology::TopologyPolicy, upgrade::plan_upgrade},
    ops::{
        ic::mgmt::MgmtOps, runtime::wasm::WasmOps, storage::registry::subnet::SubnetRegistryOps,
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
    /// Create and install a new canister.
    Create {
        role: CanisterRole,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    },

    /// Upgrade an existing canister in place.
    Upgrade { pid: Principal },
}

///
/// CanisterLifecycleResult
/// Result of a lifecycle operation.
///
/// Only creation produces a new canister identifier; other
/// lifecycle events mutate existing canisters in place.
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
/// Orchestrates canister creation and upgrade workflows.
///
/// This workflow:
/// - enforces topology invariants
/// - delegates provisioning and upgrades
/// - updates registry state
/// - triggers post-change propagation
///

pub struct CanisterLifecycleWorkflow;

impl CanisterLifecycleWorkflow {
    /// Apply a lifecycle event and return its result.
    pub(crate) async fn apply(
        event: CanisterLifecycleEvent,
    ) -> Result<CanisterLifecycleResult, Error> {
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
    ) -> Result<CanisterLifecycleResult, Error> {
        // Validate parent exists before provisioning.
        let registry_snapshot = SubnetRegistryOps::snapshot();
        TopologyPolicy::assert_parent_exists(&registry_snapshot, parent)?;

        // Provision and install the new canister.
        let pid = ProvisionWorkflow::create_and_install_canister(&role, parent, extra_arg).await?;

        // Re-read registry and validate immediate parent relationship.
        let registry_snapshot = SubnetRegistryOps::snapshot();
        TopologyPolicy::assert_immediate_parent(&registry_snapshot, pid, parent)?;

        // Propagate topology and state changes.
        PropagationWorkflow::propagate_topology(pid).await?;
        PropagationWorkflow::propagate_state(&role).await?;

        Ok(CanisterLifecycleResult::created(pid))
    }

    // ───────────────────────── Upgrade ──────────────────────────

    async fn apply_upgrade(pid: Principal) -> Result<CanisterLifecycleResult, Error> {
        let registry_snapshot = SubnetRegistryOps::snapshot();
        let entry = TopologyPolicy::registry_entry(&registry_snapshot, pid)?;

        // Load target wasm and compute upgrade plan.
        let wasm = WasmOps::try_get(&entry.role)?;
        let target_hash = wasm.module_hash();

        let status = MgmtOps::canister_status(pid).await?;
        let plan = plan_upgrade(status.module_hash, target_hash.clone());

        // Validate parent relationship if present.
        if let Some(parent_pid) = entry.parent_pid {
            TopologyPolicy::assert_parent_exists(&registry_snapshot, parent_pid)?;
            TopologyPolicy::assert_immediate_parent(&registry_snapshot, pid, parent_pid)?;
        }

        // Fast path: already running target module.
        if !plan.should_upgrade {
            log!(
                Topic::CanisterLifecycle,
                Info,
                "canister_upgrade: {pid} already running target module"
            );

            SubnetRegistryOps::update_module_hash(pid, target_hash.clone());

            let registry_snapshot = SubnetRegistryOps::snapshot();
            TopologyPolicy::assert_module_hash(&registry_snapshot, pid, target_hash)?;

            return Ok(CanisterLifecycleResult::default());
        }

        // Perform upgrade and persist new module hash.
        MgmtOps::upgrade_canister(pid, wasm.bytes()).await?;
        SubnetRegistryOps::update_module_hash(pid, target_hash.clone());

        let registry_snapshot = SubnetRegistryOps::snapshot();
        TopologyPolicy::assert_module_hash(&registry_snapshot, pid, target_hash)?;

        Ok(CanisterLifecycleResult::default())
    }
}
