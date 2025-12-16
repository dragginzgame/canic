use crate::{
    Error, ThisError,
    cdk::{api::canister_self, mgmt::CanisterInstallMode, types::Principal},
    ids::CanisterRole,
    log,
    log::Topic,
    ops::{
        ic::{
            IcOpsError, install_canic_code,
            provision::{
                build_nonroot_init_payload, create_and_install_canister, delete_canister,
                rebuild_directories_from_registry,
            },
            upgrade_canister,
        },
        orchestration::{
            OrchestrationOpsError,
            cascade::{state::root_cascade_state, topology::root_cascade_topology_for_pid},
        },
        storage::{
            directory::{AppDirectoryOps, SubnetDirectoryOps},
            topology::subnet::SubnetCanisterRegistryOps,
        },
        subsystem::reserve::{
            CanisterReserveOps, reserve_export_canister, reserve_import_canister,
            reserve_recycle_canister,
        },
        wasm::WasmOps,
    },
};

#[derive(Debug, ThisError)]
pub enum OrchestratorOpsError {
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

    #[error("canister {0} unexpectedly present in reserve")]
    InReserve(Principal),

    #[error("expected canister {0} to be in reserve")]
    NotInReserve(Principal),

    #[error("cannot perform init-based install for root canister {0}")]
    RootInitNotSupported(Principal),

    #[error("cannot build init payload for {0}: missing parent pid")]
    MissingParentPid(Principal),

    #[error(transparent)]
    IcOpsError(#[from] IcOpsError),
}

impl From<OrchestratorOpsError> for Error {
    fn from(err: OrchestratorOpsError) -> Self {
        OrchestrationOpsError::from(err).into()
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

    /// Adopt a reserve canister into topology under `parent`.
    /// Reserve export is a handoff; this event performs the attach + install.
    AdoptReserve {
        pid: Principal,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    },

    RecycleToReserve {
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
    #[allow(clippy::too_many_lines)]
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
            } => {
                assert_parent_exists(parent)?;

                let pid = create_and_install_canister(&role, parent, extra_arg).await?;

                assert_immediate_parent(pid, parent)?;
                assert_not_in_reserve(pid)?;

                cascade_all(Some(&role), Some(pid)).await?;

                Ok(LifecycleResult::created(pid))
            }

            // -----------------------------------------------------------------
            // DELETE (leaf-only)
            // -----------------------------------------------------------------
            LifecycleEvent::Delete { pid } => {
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

            // -----------------------------------------------------------------
            // UPGRADE
            // -----------------------------------------------------------------
            LifecycleEvent::Upgrade { pid } => {
                let entry = SubnetCanisterRegistryOps::try_get(pid)?;
                let wasm = WasmOps::try_get(&entry.role)?;

                if let Some(parent_pid) = entry.parent_pid {
                    assert_parent_exists(parent_pid)?;
                    assert_immediate_parent(pid, parent_pid)?;
                }
                assert_not_in_reserve(pid)?;

                upgrade_canister(entry.pid, wasm.bytes()).await?;
                SubnetCanisterRegistryOps::update_module_hash(entry.pid, wasm.module_hash())?;
                assert_module_hash(entry.pid, wasm.module_hash())?;

                Ok(LifecycleResult::default())
            }

            // -----------------------------------------------------------------
            // REINSTALL
            // -----------------------------------------------------------------
            LifecycleEvent::Reinstall { pid } => {
                let entry = SubnetCanisterRegistryOps::try_get(pid)?;
                if entry.role == CanisterRole::ROOT {
                    return Err(OrchestratorOpsError::RootInitNotSupported(pid).into());
                }

                let wasm = WasmOps::try_get(&entry.role)?;

                let parent_pid = entry
                    .parent_pid
                    .ok_or(OrchestratorOpsError::MissingParentPid(pid))?;
                assert_parent_exists(parent_pid)?;
                assert_immediate_parent(pid, parent_pid)?;
                assert_not_in_reserve(pid)?;

                let payload = build_nonroot_init_payload(&entry.role, parent_pid)?;
                install_canic_code(
                    CanisterInstallMode::Reinstall,
                    entry.pid,
                    wasm.bytes(),
                    payload,
                    None,
                )
                .await?;
                SubnetCanisterRegistryOps::update_module_hash(entry.pid, wasm.module_hash())?;
                assert_module_hash(entry.pid, wasm.module_hash())?;

                Ok(LifecycleResult::default())
            }

            // -----------------------------------------------------------------
            // ADOPT FROM RESERVE
            // -----------------------------------------------------------------
            LifecycleEvent::AdoptReserve {
                pid,
                parent,
                extra_arg,
            } => {
                // Must currently be in reserve
                assert_in_reserve(pid)?;
                assert_parent_exists(parent)?;

                // Export metadata from reserve (handoff)
                let (role, stored_hash) = reserve_export_canister(pid).await?;

                // No longer in reserve
                assert_not_in_reserve(pid)?;

                if role == CanisterRole::ROOT {
                    try_return_to_reserve(pid, "adopt_reserve role=ROOT").await;
                    return Err(OrchestratorOpsError::RootInitNotSupported(pid).into());
                }

                let wasm = WasmOps::try_get(&role)?;

                // Validate module hash matches what reserve expected (defensive)
                if wasm.module_hash() != stored_hash {
                    try_return_to_reserve(pid, "adopt_reserve module hash mismatch").await;
                    return Err(OrchestratorOpsError::ModuleHashMismatch(pid).into());
                }

                // Attach before install so init hooks can observe the registry; roll back on failure.
                SubnetCanisterRegistryOps::register(pid, &role, parent, stored_hash);

                let payload = match build_nonroot_init_payload(&role, parent) {
                    Ok(p) => p,
                    Err(err) => {
                        let _ = SubnetCanisterRegistryOps::remove(&pid);
                        try_return_to_reserve(pid, "adopt_reserve payload build failed").await;
                        return Err(err);
                    }
                };

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
                    try_return_to_reserve(pid, "adopt_reserve install failed").await;
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
            // -----------------------------------------------------------------
            // RECYCLE INTO RESERVE
            // -----------------------------------------------------------------
            LifecycleEvent::RecycleToReserve { pid } => {
                // Snapshot BEFORE destruction. If it wasn't in registry, that's a bug.
                let snap = snapshot_topology_required(pid)?;

                reserve_recycle_canister(pid).await?;

                let topology_target = snap.parent_pid.filter(|p| *p != root_pid);
                cascade_all(Some(&snap.role), topology_target).await?;

                Ok(LifecycleResult {
                    new_canister_pid: None,
                    cascaded_topology: topology_target.is_some(),
                    cascaded_directories: true,
                })
            }
        }
    }
}

//
// Topology snapshotting: single source of parent/role for destructive operations.
//

struct TopologySnapshot {
    role: CanisterRole,
    parent_pid: Option<Principal>,
}

fn snapshot_topology_required(pid: Principal) -> Result<TopologySnapshot, OrchestratorOpsError> {
    let entry = SubnetCanisterRegistryOps::get(pid)
        .ok_or(OrchestratorOpsError::RegistryEntryMissing(pid))?;

    Ok(TopologySnapshot {
        role: entry.role,
        parent_pid: entry.parent_pid,
    })
}

//
// Cascades
//

async fn cascade_all(
    role: Option<&CanisterRole>,
    topology_target: Option<Principal>,
) -> Result<(), Error> {
    if let Some(target) = topology_target {
        root_cascade_topology_for_pid(target).await?;
    }

    if let Some(ty) = role {
        let bundle = rebuild_directories_from_registry(Some(ty)).await?;
        root_cascade_state(bundle).await?;
        assert_directories_match_registry()?;
    }

    Ok(())
}

//
// Invariants
//

fn assert_parent_exists(parent_pid: Principal) -> Result<(), OrchestratorOpsError> {
    SubnetCanisterRegistryOps::get(parent_pid)
        .ok_or(OrchestratorOpsError::ParentNotFound(parent_pid))?;
    Ok(())
}

fn assert_no_children(pid: Principal) -> Result<(), OrchestratorOpsError> {
    let subtree = SubnetCanisterRegistryOps::subtree(pid);
    if subtree.len() > 1 {
        return Err(OrchestratorOpsError::SubtreeNotEmpty {
            pid,
            size: subtree.len(),
        });
    }
    Ok(())
}

fn assert_module_hash(pid: Principal, expected_hash: Vec<u8>) -> Result<(), OrchestratorOpsError> {
    let entry = SubnetCanisterRegistryOps::get(pid)
        .ok_or(OrchestratorOpsError::RegistryEntryMissing(pid))?;
    if entry.module_hash == Some(expected_hash) {
        Ok(())
    } else {
        Err(OrchestratorOpsError::ModuleHashMismatch(pid))
    }
}

fn assert_directories_match_registry() -> Result<(), OrchestratorOpsError> {
    let app_built = AppDirectoryOps::root_build_view();
    let app_exported = AppDirectoryOps::export();
    if app_built != app_exported {
        return Err(OrchestratorOpsError::AppDirectoryDiverged);
    }

    let subnet_built = SubnetDirectoryOps::root_build_view();
    let subnet_exported = SubnetDirectoryOps::export();
    if subnet_built != subnet_exported {
        return Err(OrchestratorOpsError::SubnetDirectoryDiverged);
    }

    Ok(())
}

fn assert_not_in_reserve(pid: Principal) -> Result<(), OrchestratorOpsError> {
    if CanisterReserveOps::contains(&pid) {
        Err(OrchestratorOpsError::InReserve(pid))
    } else {
        Ok(())
    }
}

fn assert_in_reserve(pid: Principal) -> Result<(), OrchestratorOpsError> {
    if CanisterReserveOps::contains(&pid) {
        Ok(())
    } else {
        Err(OrchestratorOpsError::NotInReserve(pid))
    }
}

fn assert_immediate_parent(
    pid: Principal,
    expected_parent: Principal,
) -> Result<(), OrchestratorOpsError> {
    let entry = SubnetCanisterRegistryOps::get(pid)
        .ok_or(OrchestratorOpsError::RegistryEntryMissing(pid))?;

    match entry.parent_pid {
        Some(pp) if pp == expected_parent => Ok(()),
        other => Err(OrchestratorOpsError::ImmediateParentMismatch {
            pid,
            expected: expected_parent,
            found: other,
        }),
    }
}

async fn try_return_to_reserve(pid: Principal, context: &str) {
    if let Err(err) = reserve_import_canister(pid).await {
        log!(
            Topic::CanisterLifecycle,
            Warn,
            "failed to return {pid} to reserve after {context}: {err}"
        );
    }
}
