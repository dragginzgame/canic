use crate::{
    Error,
    domain::policy::{topology::TopologyPolicy, upgrade::plan_upgrade},
    ops::{
        ic::mgmt::MgmtOps,
        runtime::wasm::WasmOps,
        storage::{
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::{
        cascade::{state::root_cascade_state, topology::root_cascade_topology_for_pid},
        ic::provision::ProvisionWorkflow,
        lifecycle::{LifecycleEvent, LifecycleResult},
        prelude::*,
    },
};

///
/// LifecycleOrchestrator
///

pub struct LifecycleOrchestrator;

impl LifecycleOrchestrator {
    pub(crate) async fn apply(event: LifecycleEvent) -> Result<LifecycleResult, Error> {
        match event {
            // -----------------------------------------------------------------
            // CREATE
            // -----------------------------------------------------------------
            LifecycleEvent::Create {
                role,
                parent,
                extra_arg,
            } => Self::apply_create(role, parent, extra_arg).await,

            // -----------------------------------------------------------------
            // UPGRADE
            // -----------------------------------------------------------------
            LifecycleEvent::Upgrade { pid } => Self::apply_upgrade(pid).await,
        }
    }

    async fn apply_create(
        role: CanisterRole,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<LifecycleResult, Error> {
        let registry_snapshot = SubnetRegistryOps::snapshot();
        TopologyPolicy::assert_parent_exists(&registry_snapshot, parent)?;

        let pid = ProvisionWorkflow::create_and_install_canister(&role, parent, extra_arg).await?;

        let registry_snapshot = SubnetRegistryOps::snapshot();
        TopologyPolicy::assert_immediate_parent(&registry_snapshot, pid, parent)?;

        cascade_all(Some(&role), Some(pid)).await?;

        Ok(LifecycleResult::created(pid))
    }

    async fn apply_upgrade(pid: Principal) -> Result<LifecycleResult, Error> {
        let registry_snapshot = SubnetRegistryOps::snapshot();
        let entry = TopologyPolicy::registry_entry(&registry_snapshot, pid)?;

        let wasm = WasmOps::try_get(&entry.role)?;
        let target_hash = wasm.module_hash();
        let status = MgmtOps::canister_status(pid).await?;
        let plan = plan_upgrade(status.module_hash, target_hash.clone());

        if let Some(parent_pid) = entry.parent_pid {
            TopologyPolicy::assert_parent_exists(&registry_snapshot, parent_pid)?;
            TopologyPolicy::assert_immediate_parent(&registry_snapshot, pid, parent_pid)?;
        }

        if !plan.should_upgrade {
            log!(
                Topic::CanisterLifecycle,
                Info,
                "canister_upgrade: {pid} already running target module"
            );
            SubnetRegistryOps::update_module_hash(pid, target_hash.clone());
            let registry_snapshot = SubnetRegistryOps::snapshot();
            TopologyPolicy::assert_module_hash(&registry_snapshot, pid, target_hash)?;

            return Ok(LifecycleResult::default());
        }

        MgmtOps::upgrade_canister(pid, wasm.bytes()).await?;
        SubnetRegistryOps::update_module_hash(pid, target_hash.clone());
        let registry_snapshot = SubnetRegistryOps::snapshot();
        TopologyPolicy::assert_module_hash(&registry_snapshot, pid, target_hash)?;

        Ok(LifecycleResult::default())
    }
}

//
// Cascades
//

async fn cascade_all(
    role_opt: Option<&CanisterRole>,
    topology_target: Option<Principal>,
) -> Result<(), Error> {
    if let Some(target) = topology_target {
        root_cascade_topology_for_pid(target).await?;
    }

    if let Some(role) = role_opt {
        // Ensure newly created/adopted canisters inherit the current app
        // and subnet states
        let snapshot = ProvisionWorkflow::rebuild_directories_from_registry(Some(role))
            .await?
            .with_app_state()
            .with_subnet_state()
            .build();

        root_cascade_state(&snapshot).await?;
        let registry_snapshot = SubnetRegistryOps::snapshot();
        let app_snapshot = AppDirectoryOps::snapshot();
        let subnet_snapshot = SubnetDirectoryOps::snapshot();
        TopologyPolicy::assert_directories_match_registry(
            &registry_snapshot,
            &app_snapshot,
            &subnet_snapshot,
        )
        .map_err(Error::from)?;
    }

    Ok(())
}
