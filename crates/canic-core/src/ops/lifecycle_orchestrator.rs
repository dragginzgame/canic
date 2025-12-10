use crate::{
    Error,
    cdk::mgmt::CanisterInstallMode,
    ids::CanisterRole,
    interface::ic::{canister::upgrade_canister, install_code},
    ops::{
        canister::{create_and_install_canister, delete_canister, sync_directories_from_registry},
        model::memory::topology::subnet::SubnetCanisterRegistryOps,
        sync::topology::root_cascade_topology_for_pid,
        wasm::WasmOps,
    },
    types::Principal,
};

/// Lifecycle events handled by the orchestrator.
pub enum LifecycleEvent {
    Create {
        ty: CanisterRole,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    },
    Delete {
        pid: Principal,
    },
    Upgrade {
        pid: Principal,
    },
    Reinstall {
        pid: Principal,
    },
}

/// Result of applying a lifecycle event.
#[derive(Default)]
pub struct LifecycleResult {
    pub new_canister_pid: Option<Principal>,
    pub cascaded_topology: bool,
    pub cascaded_directories: bool,
}

impl LifecycleResult {
    #[must_use]
    pub fn created(pid: Principal) -> Self {
        Self {
            new_canister_pid: Some(pid),
            cascaded_topology: true,
            cascaded_directories: true,
        }
    }
}

/// Single entry point for canister lifecycle orchestration.
pub struct CanisterLifecycleOrchestrator;

impl CanisterLifecycleOrchestrator {
    /// Apply a lifecycle event and return its result.
    pub async fn apply(event: LifecycleEvent) -> Result<LifecycleResult, Error> {
        match event {
            LifecycleEvent::Create {
                ty,
                parent,
                extra_arg,
            } => {
                let pid = create_and_install_canister(&ty, parent, extra_arg).await?;
                // topology + directories handled here to keep cascades centralized
                root_cascade_topology_for_pid(pid).await?;
                sync_directories_from_registry(Some(&ty)).await?;
                Ok(LifecycleResult::created(pid))
            }
            LifecycleEvent::Delete { pid } => {
                let (removed_ty, parent_pid) = delete_canister(pid).await?;

                if let Some(parent_pid) = parent_pid {
                    // Cascade the branch rooted at parent to drop the child
                    root_cascade_topology_for_pid(parent_pid).await?;
                }

                if let Some(ty) = removed_ty {
                    sync_directories_from_registry(Some(&ty)).await?;
                }

                Ok(LifecycleResult {
                    new_canister_pid: None,
                    cascaded_topology: parent_pid.is_some(),
                    cascaded_directories: removed_ty.is_some(),
                })
            }
            LifecycleEvent::Upgrade { pid } => {
                let entry = SubnetCanisterRegistryOps::try_get(pid)?;
                let wasm = WasmOps::try_get(&entry.ty)?;

                upgrade_canister(entry.pid, wasm.bytes()).await?;
                SubnetCanisterRegistryOps::update_module_hash(entry.pid, wasm.module_hash())?;

                Ok(LifecycleResult {
                    new_canister_pid: None,
                    cascaded_topology: false,
                    cascaded_directories: false,
                })
            }
            LifecycleEvent::Reinstall { pid } => {
                let entry = SubnetCanisterRegistryOps::try_get(pid)?;
                let wasm = WasmOps::try_get(&entry.ty)?;

                install_code(CanisterInstallMode::Reinstall, entry.pid, wasm.bytes(), ()).await?;
                SubnetCanisterRegistryOps::update_module_hash(entry.pid, wasm.module_hash())?;

                Ok(LifecycleResult {
                    new_canister_pid: None,
                    cascaded_topology: false,
                    cascaded_directories: false,
                })
            }
        }
    }
}
