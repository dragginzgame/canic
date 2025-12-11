use crate::{
    Error,
    cdk::{api::canister_self, mgmt::CanisterInstallMode},
    ids::CanisterRole,
    interface::ic::{canister::upgrade_canister, install_code},
    ops::{
        mgmt::{
            ProvisioningError, create_and_install_canister, delete_canister,
            rebuild_directories_from_registry,
        },
        model::memory::{
            directory::{AppDirectoryOps, SubnetDirectoryOps},
            reserve::{CanisterReserveOps, reserve_export_canister, reserve_recycle_canister},
            topology::subnet::SubnetCanisterRegistryOps,
        },
        sync::{state::root_cascade_state, topology::root_cascade_topology_for_pid},
        wasm::WasmOps,
    },
    types::Principal,
};

///
/// LifecycleEvent
/// Lifecycle events handled by the orchestrator
///

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
    AdoptReserve {
        pid: Principal,
    },
    RecycleToReserve {
        pid: Principal,
    },
}

///
/// LifecycleResult
/// Result of applying a lifecycle event.
///

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

/// Main orchestrator.
pub struct CanisterLifecycleOrchestrator;

impl CanisterLifecycleOrchestrator {
    /// Entry point for lifecycle orchestration.
    #[allow(clippy::too_many_lines)]
    pub async fn apply(event: LifecycleEvent) -> Result<LifecycleResult, Error> {
        // Root PID for filtering topology cascades that would otherwise target root.
        let root_pid = canister_self();

        match event {
            // -----------------------------------------------------------------
            // CREATE
            // -----------------------------------------------------------------
            LifecycleEvent::Create {
                ty,
                parent,
                extra_arg,
            } => {
                assert_parent_exists(parent)?;

                let pid = match create_and_install_canister(&ty, parent, extra_arg).await {
                    Ok(pid) => pid,
                    Err(ProvisioningError::InstallFailed { pid, source }) => {
                        let _ = reserve_recycle_canister(pid).await;
                        return Err(source);
                    }
                    Err(ProvisioningError::Other(err)) => return Err(err),
                };

                assert_registry_role(pid, &ty)?;
                assert_registry_parent(pid, parent)?;
                assert_immediate_parent(pid, parent)?;
                assert_not_in_reserve(pid)?;

                // Topology: targeted cascade rooted at the newly created canister.
                // This is always non-root.
                cascade_all(Some(&ty), Some(pid)).await?;

                Ok(LifecycleResult::created(pid))
            }

            // -----------------------------------------------------------------
            // DELETE
            // -----------------------------------------------------------------
            LifecycleEvent::Delete { pid } => {
                // New invariant: can only delete leaves
                assert_no_children(pid)?;

                let (removed_ty, parent_pid) = delete_canister(pid).await?;

                // Topology cascade
                // Use parent as target, unless parent is root (root never cascades)
                let topology_target = parent_pid.filter(|p| *p != root_pid);

                cascade_all(
                    removed_ty.as_ref(), // directory cascade triggered only if this type participates
                    topology_target,     // targeted topology cascade
                )
                .await?;

                Ok(LifecycleResult {
                    new_canister_pid: None,
                    cascaded_topology: topology_target.is_some(),
                    cascaded_directories: removed_ty.is_some(),
                })
            }

            // -----------------------------------------------------------------
            // UPGRADE
            // -----------------------------------------------------------------
            LifecycleEvent::Upgrade { pid } => {
                let entry = SubnetCanisterRegistryOps::try_get(pid)?;
                let wasm = WasmOps::try_get(&entry.ty)?;

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
                let wasm = WasmOps::try_get(&entry.ty)?;

                if let Some(parent_pid) = entry.parent_pid {
                    assert_parent_exists(parent_pid)?;
                    assert_immediate_parent(pid, parent_pid)?;
                }
                assert_not_in_reserve(pid)?;

                install_code(CanisterInstallMode::Reinstall, entry.pid, wasm.bytes(), ()).await?;
                SubnetCanisterRegistryOps::update_module_hash(entry.pid, wasm.module_hash())?;
                assert_module_hash(entry.pid, wasm.module_hash())?;

                Ok(LifecycleResult::default())
            }

            // -----------------------------------------------------------------
            // ADOPT FROM RESERVE
            // -----------------------------------------------------------------
            LifecycleEvent::AdoptReserve { pid } => {
                // After export, `pid` is attached to a parent in the registry.
                let (ty, parent_pid) = reserve_export_canister(pid).await?;

                assert_not_in_reserve(pid)?;
                assert_parent_exists(parent_pid)?;
                assert_immediate_parent(pid, parent_pid)?;

                // Topology: targeted cascade on the adopted canister’s subtree.
                // This is always non-root.
                cascade_all(Some(&ty), Some(pid)).await?;

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
                // After recycle, `pid` is removed from the active topology; it may
                // no longer appear in the registry subtree.
                let (ty, parent_pid) = reserve_recycle_canister(pid).await?;

                // Topology: same reasoning as DELETE — target the parent subtree,
                // but only if parent is non-root. Root is never a cascade target.
                let topology_target = parent_pid.filter(|p| *p != root_pid);

                cascade_all(ty.as_ref(), topology_target).await?;

                Ok(LifecycleResult {
                    new_canister_pid: None,
                    cascaded_topology: topology_target.is_some(),
                    cascaded_directories: ty.is_some(),
                })
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Unified Cascade Logic
// -----------------------------------------------------------------------------

/// Perform topology + directories + state cascades in correct order.
///
/// - `ty` controls whether directory rebuild is required.
/// - `topology_target` is the canister whose branch changed and should be
///   used as the *target* of a **targeted** topology cascade. This must never
///   be the root PID.
async fn cascade_all(
    ty: Option<&CanisterRole>,
    topology_target: Option<Principal>,
) -> Result<(), Error> {
    // Topology: targeted cascade only, never full-root.
    if let Some(target) = topology_target {
        root_cascade_topology_for_pid(target).await?;
    }

    // Directories + state: driven by type; this can be global-ish, but is
    // independent of topology targeting semantics.
    if let Some(ty) = ty {
        let bundle = rebuild_directories_from_registry(Some(ty)).await?;
        root_cascade_state(bundle).await?;
        assert_directories_match_registry()?;
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// Invariants
// -----------------------------------------------------------------------------

fn assert_parent_exists(parent_pid: Principal) -> Result<(), Error> {
    SubnetCanisterRegistryOps::get(parent_pid)
        .ok_or_else(|| Error::custom(format!("parent {parent_pid} not found in registry")))?;
    Ok(())
}

fn assert_registry_role(pid: Principal, expected_ty: &CanisterRole) -> Result<(), Error> {
    let entry = SubnetCanisterRegistryOps::get(pid)
        .ok_or_else(|| Error::custom(format!("registry entry missing for {pid}")))?;
    if &entry.ty == expected_ty {
        Ok(())
    } else {
        Err(Error::custom(format!(
            "registry type mismatch for {pid}: expected {expected_ty}, found {}",
            entry.ty
        )))
    }
}

fn assert_registry_parent(pid: Principal, expected_parent: Principal) -> Result<(), Error> {
    let entry = SubnetCanisterRegistryOps::get(pid)
        .ok_or_else(|| Error::custom(format!("registry entry missing for {pid}")))?;
    if entry.parent_pid == Some(expected_parent) {
        Ok(())
    } else {
        Err(Error::custom(format!(
            "registry parent mismatch for {pid}: expected {expected_parent}, found {:?}",
            entry.parent_pid
        )))
    }
}

/// Verify that the canister’s immediate parent matches expectations. This is
/// stricter and clearer than subtree-membership checks.
fn assert_immediate_parent(pid: Principal, expected_parent: Principal) -> Result<(), Error> {
    let entry = SubnetCanisterRegistryOps::get(pid)
        .ok_or_else(|| Error::custom(format!("registry entry missing for {pid}")))?;
    match entry.parent_pid {
        Some(pp) if pp == expected_parent => Ok(()),
        other => Err(Error::custom(format!(
            "immediate-parent mismatch: canister {pid} expects parent {expected_parent}, got {other:?}",
        ))),
    }
}

fn assert_no_children(pid: Principal) -> Result<(), Error> {
    let subtree = SubnetCanisterRegistryOps::subtree(pid);

    // subtree always contains the node itself
    if subtree.len() > 1 {
        return Err(Error::custom(format!(
            "cannot delete {pid}: subtree is not empty ({} nodes)",
            subtree.len()
        )));
    }

    Ok(())
}

fn assert_module_hash(pid: Principal, expected_hash: Vec<u8>) -> Result<(), Error> {
    let entry = SubnetCanisterRegistryOps::get(pid)
        .ok_or_else(|| Error::custom(format!("registry entry missing for {pid}")))?;
    if entry.module_hash == Some(expected_hash) {
        Ok(())
    } else {
        Err(Error::custom(format!("module hash mismatch for {pid}")))
    }
}

fn assert_directories_match_registry() -> Result<(), Error> {
    let app_built = AppDirectoryOps::root_build_view();
    let app_exported = AppDirectoryOps::export();
    if app_built != app_exported {
        return Err(Error::custom("app directory diverged from registry"));
    }

    let subnet_built = SubnetDirectoryOps::root_build_view();
    let subnet_exported = SubnetDirectoryOps::export();
    if subnet_built != subnet_exported {
        return Err(Error::custom("subnet directory diverged from registry"));
    }

    Ok(())
}

fn assert_not_in_reserve(pid: Principal) -> Result<(), Error> {
    if CanisterReserveOps::contains(&pid) {
        Err(Error::custom(format!(
            "canister {pid} unexpectedly present in reserve"
        )))
    } else {
        Ok(())
    }
}
