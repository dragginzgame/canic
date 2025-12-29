pub mod admin;
pub mod admissibility;
pub mod scheduler;

use crate::{
    Error,
    cdk::{
        mgmt::{CanisterSettings, UpdateSettingsArgs},
        types::{Cycles, TC},
    },
    dto::pool::{CanisterPoolStatusView, PoolImportSummary, PoolStatusCounts},
    log,
    log::Topic,
    ops::{
        ic::{
            get_cycles,
            mgmt::{create_canister, uninstall_code},
            update_settings,
        },
        storage::{
            pool::{PoolOps, pool_controllers},
            registry::SubnetRegistryOps,
        },
    },
    policy::{self, pool::PoolPolicyError},
};
use candid::Principal;

/// Default cycles allocated to freshly created pool canisters.
const POOL_CANISTER_CYCLES: u128 = 5 * TC;

// -----------------------------------------------------------------------------
// Reset
// -----------------------------------------------------------------------------

pub async fn reset_into_pool(pid: Principal) -> Result<Cycles, Error> {
    update_settings(&UpdateSettingsArgs {
        canister_id: pid,
        settings: CanisterSettings {
            controllers: Some(pool_controllers()),
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
// Creation
// -----------------------------------------------------------------------------

pub async fn pool_create_canister() -> Result<Principal, Error> {
    policy::pool::authority::require_pool_admin()?;

    let cycles = Cycles::new(POOL_CANISTER_CYCLES);
    let pid = create_canister(pool_controllers(), cycles.clone()).await?;

    PoolOps::register_ready(pid, cycles, None, None, None);

    Ok(pid)
}

// -----------------------------------------------------------------------------
// Import
// -----------------------------------------------------------------------------

pub async fn pool_import_canister(pid: Principal) -> Result<(), Error> {
    policy::pool::authority::require_pool_admin()?;
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
    policy::pool::authority::require_pool_admin()?;

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

pub async fn pool_import_queued_canisters(
    pids: Vec<Principal>,
) -> Result<(u64, u64, u64, u64, PoolImportSummary), Error> {
    policy::pool::authority::require_pool_admin()?;

    let mut summary = PoolImportSummary::default();
    let total = pids.len() as u64;

    let mut added = 0;
    let mut requeued = 0;
    let mut skipped = 0;

    for pid in pids {
        match admissibility::check_can_enter_pool(pid).await {
            Ok(()) => {
                if let Some(entry) = PoolOps::get_view(pid) {
                    if matches!(entry.status, CanisterPoolStatusView::Failed { .. }) {
                        mark_pending_reset(pid);
                        requeued += 1;
                    } else {
                        skipped += 1;
                        // already ready or pending
                    }
                } else {
                    mark_pending_reset(pid);
                    added += 1;
                }
            }

            Err(PoolPolicyError::RegisteredInSubnet(_)) => {
                skipped += 1;
                summary.skipped_in_registry += 1;
            }

            Err(PoolPolicyError::NonImportableOnLocal { .. }) => {
                skipped += 1;
                summary.skipped_non_importable += 1;
            }

            Err(_) => {
                skipped += 1;
            }
        }
    }

    summary.status_counts = pool_status_counts();

    Ok((added, requeued, skipped, total, summary))
}

// -----------------------------------------------------------------------------
// Export
// -----------------------------------------------------------------------------

pub async fn pool_export_canister(
    pid: Principal,
) -> Result<(crate::ids::CanisterRole, Vec<u8>), Error> {
    policy::pool::authority::require_pool_admin()?;

    let entry = PoolOps::get_view(pid).ok_or(PoolPolicyError::NotReadyForExport)?;

    let is_ready = matches!(entry.status, CanisterPoolStatusView::Ready);
    let (role, hash) = policy::pool::export::can_export(is_ready, entry.role, entry.module_hash)?;

    let _ = PoolOps::take(&pid);

    Ok((role, hash))
}

// -----------------------------------------------------------------------------
// Stats
// -----------------------------------------------------------------------------

#[must_use]
pub fn pool_status_counts() -> PoolStatusCounts {
    let mut counts = PoolStatusCounts::default();

    for (_, entry) in PoolOps::export_view() {
        match entry.status {
            CanisterPoolStatusView::Ready => counts.ready += 1,
            CanisterPoolStatusView::PendingReset => counts.pending_reset += 1,
            CanisterPoolStatusView::Failed { .. } => counts.failed += 1,
        }
    }

    counts.total = counts.ready + counts.pending_reset + counts.failed;
    counts
}
