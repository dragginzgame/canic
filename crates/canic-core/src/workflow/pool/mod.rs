pub mod admin;
pub mod admissibility;
pub mod scheduler;

use crate::{
    Error,
    cdk::{
        mgmt::{CanisterSettings, UpdateSettingsArgs},
        types::TC,
    },
    dto::pool::{CanisterPoolStatusView, PoolBatchResult},
    ops::{
        ic::{create_canister, get_cycles, uninstall_code, update_settings},
        runtime::env::EnvOps,
        storage::{
            pool::{PoolOps, pool_controllers},
            registry::SubnetRegistryOps,
        },
    },
    policy::{self, pool::PoolPolicyError},
    workflow::prelude::*,
};

/// Default cycles allocated to freshly created pool canisters.
const POOL_CANISTER_CYCLES: u128 = 5 * TC;

// -----------------------------------------------------------------------------
// Reset
// -----------------------------------------------------------------------------

pub async fn reset_into_pool(pid: Principal) -> Result<Cycles, Error> {
    update_settings(&UpdateSettingsArgs {
        canister_id: pid,
        settings: CanisterSettings {
            controllers: Some(pool_controllers()?),
            ..Default::default()
        },
    })
    .await?;

    uninstall_code(pid).await?;
    get_cycles(pid).await
}

// -----------------------------------------------------------------------------
// Metadata helpers
// -----------------------------------------------------------------------------

fn mark_pending_reset(pid: Principal) {
    PoolOps::mark_pending_reset(pid);
}

fn mark_ready(pid: Principal, cycles: Cycles) {
    PoolOps::mark_ready(pid, cycles);
}

fn mark_failed(pid: Principal, err: &Error) {
    PoolOps::mark_failed(pid, err);
}

// -----------------------------------------------------------------------------
// Auth
// -----------------------------------------------------------------------------

fn require_pool_admin() -> Result<(), Error> {
    EnvOps::require_root()?;

    Ok(())
}

// -----------------------------------------------------------------------------
// Creation
// -----------------------------------------------------------------------------

pub async fn pool_create_canister() -> Result<Principal, Error> {
    require_pool_admin()?;

    let cycles = Cycles::new(POOL_CANISTER_CYCLES);
    let pid = create_canister(pool_controllers()?, cycles.clone()).await?;

    PoolOps::register_ready(pid, cycles, None, None, None);

    Ok(pid)
}

// -----------------------------------------------------------------------------
// Import
// -----------------------------------------------------------------------------

pub async fn pool_import_canister(pid: Principal) -> Result<(), Error> {
    require_pool_admin()?;
    admissibility::check_can_enter_pool(pid).await?;

    mark_pending_reset(pid);

    match reset_into_pool(pid).await {
        Ok(cycles) => {
            let _ = SubnetRegistryOps::remove(&pid);
            mark_ready(pid, cycles);
            Ok(())
        }
        Err(err) => {
            log!(
                Topic::CanisterPool,
                Warn,
                "pool import failed for {pid}: {err}"
            );
            mark_failed(pid, &err);
            Err(err)
        }
    }
}

// -----------------------------------------------------------------------------
// Recycle
// -----------------------------------------------------------------------------

pub async fn pool_recycle_canister(pid: Principal) -> Result<(), Error> {
    require_pool_admin()?;

    // Must exist in registry to be recycled
    let entry = SubnetRegistryOps::get(pid).ok_or(PoolPolicyError::NotReadyForExport)?;

    let role = Some(entry.role.clone());
    let module_hash = entry.module_hash.clone();

    // Destructive reset
    let cycles = reset_into_pool(pid).await?;

    // Remove from topology
    let _ = SubnetRegistryOps::remove(&pid);

    // Register back into pool, preserving metadata
    PoolOps::register_ready(pid, cycles, role, None, module_hash);

    Ok(())
}

// -----------------------------------------------------------------------------
// Bulk import
// -----------------------------------------------------------------------------

pub async fn pool_import_queued_canisters(pids: Vec<Principal>) -> Result<PoolBatchResult, Error> {
    require_pool_admin()?;

    let total = pids.len() as u64;

    let mut added = 0;
    let mut requeued = 0;
    let mut skipped = 0;

    for pid in pids {
        match admissibility::check_can_enter_pool(pid).await {
            Ok(()) => {
                if let Some(entry) = PoolOps::get_view(pid) {
                    match entry.status {
                        CanisterPoolStatusView::Failed { .. } => {
                            mark_pending_reset(pid);
                            requeued += 1;
                        }
                        _ => {
                            // already ready or pending reset
                            skipped += 1;
                        }
                    }
                } else {
                    mark_pending_reset(pid);
                    added += 1;
                }
            }

            // Any policy rejection is treated as a skip
            Err(_) => {
                skipped += 1;
            }
        }
    }

    let result = PoolBatchResult {
        total,
        added,
        requeued,
        skipped,
    };

    if result.added > 0 || result.requeued > 0 {
        scheduler::schedule();
    }

    Ok(result)
}

// -----------------------------------------------------------------------------
// Export
// -----------------------------------------------------------------------------

pub async fn pool_export_canister(
    pid: Principal,
) -> Result<(crate::ids::CanisterRole, Vec<u8>), Error> {
    require_pool_admin()?;

    let entry = PoolOps::get_view(pid).ok_or(PoolPolicyError::NotReadyForExport)?;

    let is_ready = matches!(entry.status, CanisterPoolStatusView::Ready);
    let (role, hash) = policy::pool::export::can_export(is_ready, entry.role, entry.module_hash)?;

    let _ = PoolOps::take(&pid);

    Ok((role, hash))
}
