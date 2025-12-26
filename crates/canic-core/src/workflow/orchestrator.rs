use crate::{
    Error, ThisError,
    cdk::{api::canister_self, mgmt::CanisterInstallMode, types::Principal},
    ids::CanisterRole,
    log,
    log::Topic,
    ops::{
        canister::install_code_with_extra_arg,
        ic::{IcOpsError, mgmt::delete_canister, upgrade_canister},
        storage::{
            directory::{AppDirectoryOps, SubnetDirectoryOps},
            topology::subnet::SubnetCanisterRegistryOps,
        },
        wasm::WasmOps,
    },
    workflow::{
        WorkflowError,
        cascade::{state::root_cascade_state, topology::root_cascade_topology_for_pid},
        ic::provision::{
            build_nonroot_init_payload, create_and_install_canister,
            rebuild_directories_from_registry,
        },
        pool::{PoolOps, pool_export_canister, pool_import_canister, pool_recycle_canister},
    },
};

///
/// OrchestratorError
///

#[derive(Debug, ThisError)]
pub enum OrchestratorError {
    #[error("parent {0} not found in registry")]
    ParentNotFound(Principal),

    #[error("registry entry missing for {0}")]
    RegistryEntryMissing(Principal),

    #[error("immediate-parent mismatch: canister {pid} expects parent {expected}, got {found:?}")]
    ImmediateParentMismatch {
        pid: Principal,
        expected: Principal,
        found: Option<Principal>,
    },

    #[error("cannot delete {pid}: subtree is not empty ({size} nodes)")]
    SubtreeNotEmpty { pid: Principal, size: usize },

    #[error("module hash mismatch for {0}")]
    ModuleHashMismatch(Principal),

    #[error("app directory diverged from registry")]
    AppDirectoryDiverged,

    #[error("subnet directory diverged from registry")]
    SubnetDirectoryDiverged,

    #[error("canister {0} unexpectedly present in pool")]
    InPool(Principal),

    #[error("expected canister {0} to be in pool")]
    NotInPool(Principal),

    #[error("cannot perform init-based install for root canister {0}")]
    RootInitNotSupported(Principal),

    #[error("cannot build init payload for {0}: missing parent pid")]
    MissingParentPid(Principal),

    #[error(transparent)]
    IcOpsError(#[from] IcOpsError),
}

impl From<OrchestratorError> for Error {
    fn from(err: OrchestratorError) -> Self {
        WorkflowError::from(err).into()
    }
}

pub enum LifecycleEvent {
    Create {
        role: CanisterRole,
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

    /// Adopt a pool canister into topology under `parent`.
    /// Pool export is a handoff; this event performs the attach + install.
    AdoptPool {
        pid: Principal,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    },

    RecycleToPool {
        pid: Principal,
    },
}

#[derive(Default)]
pub struct LifecycleResult {
    pub new_canister_pid: Option<Principal>,
    pub cascaded_topology: bool,
    pub cascaded_directories: bool,
}

impl LifecycleResult {
    #[must_use]
    pub const fn created(pid: Principal) -> Self {
        Self {
            new_canister_pid: Some(pid),
            cascaded_topology: true,
            cascaded_directories: true,
        }
    }
}

pub struct CanisterLifecycleOrchestrator;

impl CanisterLifecycleOrchestrator {
    pub async fn apply(event: LifecycleEvent) -> Result<LifecycleResult, Error> {
        let root_pid = canister_self();

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
            // DELETE (leaf-only)
            // -----------------------------------------------------------------
            LifecycleEvent::Delete { pid } => Self::apply_delete(pid, root_pid).await,

            // -----------------------------------------------------------------
            // UPGRADE
            // -----------------------------------------------------------------
            LifecycleEvent::Upgrade { pid } => Self::apply_upgrade(pid).await,

            // -----------------------------------------------------------------
            // REINSTALL
            // -----------------------------------------------------------------
            LifecycleEvent::Reinstall { pid } => Self::apply_reinstall(pid).await,

            // -----------------------------------------------------------------
            // ADOPT FROM POOL
            // -----------------------------------------------------------------
            LifecycleEvent::AdoptPool {
                pid,
                parent,
                extra_arg,
            } => Self::apply_adopt_pool(pid, parent, extra_arg).await,
            // -----------------------------------------------------------------
            // RECYCLE INTO POOL
            // -----------------------------------------------------------------
            LifecycleEvent::RecycleToPool { pid } => {
                Self::apply_recycle_to_pool(pid, root_pid).await
            }
        }
    }

    async fn apply_create(
        role: CanisterRole,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<LifecycleResult, Error> {
        assert_parent_exists(parent)?;

        let pid = create_and_install_canister(&role, parent, extra_arg).await?;

        assert_immediate_parent(pid, parent)?;
        assert_not_in_pool(pid)?;

        cascade_all(Some(&role), Some(pid)).await?;

        Ok(LifecycleResult::created(pid))
    }

    async fn apply_delete(pid: Principal, root_pid: Principal) -> Result<LifecycleResult, Error> {
        assert_no_children(pid)?;

        // Snapshot BEFORE destructive delete.
        let snap = snapshot_topology_required(pid)?;

        delete_canister(pid).await?;

        let topology_target = snap.parent_pid.filter(|p| *p != root_pid);
        cascade_all(Some(&snap.role), topology_target).await?;

        Ok(LifecycleResult {
            new_canister_pid: None,
            cascaded_topology: topology_target.is_some(),
            cascaded_directories: true,
        })
    }

    async fn apply_upgrade(pid: Principal) -> Result<LifecycleResult, Error> {
        let entry = SubnetCanisterRegistryOps::get(pid)
            .ok_or(OrchestratorError::RegistryEntryMissing(pid))?;

        let wasm = WasmOps::try_get(&entry.role)?;

        if let Some(parent_pid) = entry.parent_pid {
            assert_parent_exists(parent_pid)?;
            assert_immediate_parent(pid, parent_pid)?;
        }
        assert_not_in_pool(pid)?;

        upgrade_canister(entry.pid, wasm.bytes()).await?;
        SubnetCanisterRegistryOps::update_module_hash(entry.pid, wasm.module_hash());
        assert_module_hash(entry.pid, wasm.module_hash())?;

        Ok(LifecycleResult::default())
    }

    async fn apply_reinstall(pid: Principal) -> Result<LifecycleResult, Error> {
        let entry = SubnetCanisterRegistryOps::get(pid)
            .ok_or(OrchestratorError::RegistryEntryMissing(pid))?;

        if entry.role == CanisterRole::ROOT {
            return Err(OrchestratorError::RootInitNotSupported(pid).into());
        }

        let wasm = WasmOps::try_get(&entry.role)?;

        let parent_pid = entry
            .parent_pid
            .ok_or(OrchestratorError::MissingParentPid(pid))?;
        assert_parent_exists(parent_pid)?;
        assert_immediate_parent(pid, parent_pid)?;
        assert_not_in_pool(pid)?;

        let payload = build_nonroot_init_payload(&entry.role, parent_pid);
        install_code_with_extra_arg(
            CanisterInstallMode::Reinstall,
            entry.pid,
            wasm.bytes(),
            payload,
            None,
        )
        .await?;
        SubnetCanisterRegistryOps::update_module_hash(entry.pid, wasm.module_hash());
        assert_module_hash(entry.pid, wasm.module_hash())?;

        Ok(LifecycleResult::default())
    }

    async fn apply_adopt_pool(
        pid: Principal,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<LifecycleResult, Error> {
        // Must currently be in pool
        assert_in_pool(pid)?;
        assert_parent_exists(parent)?;

        // Export metadata from pool (handoff)
        let (role, stored_hash) = pool_export_canister(pid).await?;

        // No longer in pool
        assert_not_in_pool(pid)?;

        if role == CanisterRole::ROOT {
            try_return_to_pool(pid, "adopt_pool role=ROOT").await;
            return Err(OrchestratorError::RootInitNotSupported(pid).into());
        }

        let wasm = WasmOps::try_get(&role)?;

        // Validate module hash matches what pool expected (defensive)
        if wasm.module_hash() != stored_hash {
            try_return_to_pool(pid, "adopt_pool module hash mismatch").await;
            return Err(OrchestratorError::ModuleHashMismatch(pid).into());
        }

        // Attach before install so init hooks can observe the registry; roll back on failure.
        if let Err(err) = SubnetCanisterRegistryOps::register(pid, &role, parent, stored_hash) {
            try_return_to_pool(pid, "adopt_pool register failed").await;
            return Err(err);
        }

        let payload = build_nonroot_init_payload(&role, parent);
        if let Err(err) = install_canic_code(
            CanisterInstallMode::Install,
            pid,
            wasm.bytes(),
            payload,
            extra_arg,
        )
        .await
        {
            let _ = SubnetCanisterRegistryOps::remove(&pid);
            try_return_to_pool(pid, "adopt_pool install failed").await;
            return Err(err);
        }

        // Postconditions
        assert_immediate_parent(pid, parent)?;

        // Targeted cascade on the newly adopted canister
        cascade_all(Some(&role), Some(pid)).await?;

        Ok(LifecycleResult {
            new_canister_pid: None,
            cascaded_topology: true,
            cascaded_directories: true,
        })
    }

    async fn apply_recycle_to_pool(
        pid: Principal,
        root_pid: Principal,
    ) -> Result<LifecycleResult, Error> {
        // Snapshot BEFORE destruction. If it wasn't in registry, that's a bug.
        let snap = snapshot_topology_required(pid)?;

        pool_recycle_canister(pid).await?;

        let topology_target = snap.parent_pid.filter(|p| *p != root_pid);
        cascade_all(Some(&snap.role), topology_target).await?;

        Ok(LifecycleResult {
            new_canister_pid: None,
            cascaded_topology: topology_target.is_some(),
            cascaded_directories: true,
        })
    }
}

//
// Topology snapshotting: single source of parent/role for destructive operations.
//

struct TopologySnapshot {
    role: CanisterRole,
    parent_pid: Option<Principal>,
}

fn snapshot_topology_required(pid: Principal) -> Result<TopologySnapshot, OrchestratorError> {
    let entry =
        SubnetCanisterRegistryOps::get(pid).ok_or(OrchestratorError::RegistryEntryMissing(pid))?;

    Ok(TopologySnapshot {
        role: entry.role,
        parent_pid: entry.parent_pid,
    })
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
        // Ensure newly created/adopted canisters inherit the current app state.
        let bundle = rebuild_directories_from_registry(Some(role))
            .await
            .with_app_state();
        root_cascade_state(bundle).await?;
        assert_directories_match_registry()?;
    }

    Ok(())
}

//
// Invariants
//

fn assert_parent_exists(parent_pid: Principal) -> Result<(), OrchestratorError> {
    SubnetCanisterRegistryOps::get(parent_pid)
        .ok_or(OrchestratorError::ParentNotFound(parent_pid))?;
    Ok(())
}

fn assert_no_children(pid: Principal) -> Result<(), OrchestratorError> {
    let subtree = SubnetCanisterRegistryOps::subtree(pid);
    if subtree.len() > 1 {
        return Err(OrchestratorError::SubtreeNotEmpty {
            pid,
            size: subtree.len(),
        });
    }
    Ok(())
}

fn assert_module_hash(pid: Principal, expected_hash: Vec<u8>) -> Result<(), OrchestratorError> {
    let entry =
        SubnetCanisterRegistryOps::get(pid).ok_or(OrchestratorError::RegistryEntryMissing(pid))?;
    if entry.module_hash == Some(expected_hash) {
        Ok(())
    } else {
        Err(OrchestratorError::ModuleHashMismatch(pid))
    }
}

fn assert_directories_match_registry() -> Result<(), Error> {
    let app_built = AppDirectoryOps::root_build_view();
    let app_exported = AppDirectoryOps::export();
    if app_built != app_exported {
        return Err(OrchestratorError::AppDirectoryDiverged.into());
    }

    let subnet_built = SubnetDirectoryOps::root_build_view();
    let subnet_exported = SubnetDirectoryOps::export();
    if subnet_built != subnet_exported {
        return Err(OrchestratorError::SubnetDirectoryDiverged.into());
    }

    Ok(())
}

fn assert_not_in_pool(pid: Principal) -> Result<(), OrchestratorError> {
    if PoolOps::contains(&pid) {
        Err(OrchestratorError::InPool(pid))
    } else {
        Ok(())
    }
}

fn assert_in_pool(pid: Principal) -> Result<(), OrchestratorError> {
    if PoolOps::contains(&pid) {
        Ok(())
    } else {
        Err(OrchestratorError::NotInPool(pid))
    }
}

fn assert_immediate_parent(
    pid: Principal,
    expected_parent: Principal,
) -> Result<(), OrchestratorError> {
    let entry =
        SubnetCanisterRegistryOps::get(pid).ok_or(OrchestratorError::RegistryEntryMissing(pid))?;

    match entry.parent_pid {
        Some(pp) if pp == expected_parent => Ok(()),
        other => Err(OrchestratorError::ImmediateParentMismatch {
            pid,
            expected: expected_parent,
            found: other,
        }),
    }
}

async fn try_return_to_pool(pid: Principal, context: &str) {
    if let Err(err) = pool_import_canister(pid).await {
        log!(
            Topic::CanisterLifecycle,
            Warn,
            "failed to return {pid} to pool after {context}: {err}"
        );
    }
}
