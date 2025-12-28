pub mod admin;
pub mod admissibility;
pub mod scheduler;

use crate::{
    Error,
    cdk::mgmt::{CanisterSettings, UpdateSettingsArgs},
    dto::pool::{PoolImportSummary, PoolStatusCounts},
    log,
    log::Topic,
    model::memory::pool::CanisterPoolStatus,
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
    types::{Cycles, TC},
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

fn register_or_update(
    pid: Principal,
    cycles: Cycles,
    status: CanisterPoolStatus,
    role: Option<crate::ids::CanisterRole>,
    parent: Option<Principal>,
    module_hash: Option<Vec<u8>>,
) {
    if let Some(mut entry) = PoolOps::get(pid) {
        entry.cycles = cycles;
        entry.status = status;
        entry.role = role.or(entry.role);
        entry.parent = parent.or(entry.parent);
        entry.module_hash = module_hash.or(entry.module_hash);
        let _ = PoolOps::update(pid, entry);
    } else {
        PoolOps::register(pid, cycles, status, role, parent, module_hash);
    }
}

fn mark_pending_reset(pid: Principal) {
    register_or_update(
        pid,
        Cycles::default(),
        CanisterPoolStatus::PendingReset,
        None,
        None,
        None,
    );
}

fn mark_ready(pid: Principal, cycles: Cycles) {
    register_or_update(pid, cycles, CanisterPoolStatus::Ready, None, None, None);
}

fn mark_failed(pid: Principal, err: &Error) {
    register_or_update(
        pid,
        Cycles::default(),
        CanisterPoolStatus::Failed {
            reason: err.to_string(),
        },
        None,
        None,
        None,
    );
}

// -----------------------------------------------------------------------------
// Creation
// -----------------------------------------------------------------------------

pub async fn pool_create_canister() -> Result<Principal, Error> {
    policy::pool::authority::require_pool_admin()?;

    let cycles = Cycles::new(POOL_CANISTER_CYCLES);
    let pid = create_canister(pool_controllers(), cycles.clone()).await?;

    PoolOps::register(pid, cycles, CanisterPoolStatus::Ready, None, None, None);

    Ok(pid)
}

// -----------------------------------------------------------------------------
// Import
// -----------------------------------------------------------------------------

pub async fn pool_import_canister(pid: Principal) -> Result<(), Error> {
    policy::pool::authority::require_pool_admin()?;
    admissibility::can_enter_pool(pid).await?;

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
    PoolOps::register(
        pid,
        cycles,
        CanisterPoolStatus::Ready,
        role,
        None,
        module_hash,
    );

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
        match admissibility::can_enter_pool(pid).await {
            Ok(()) => {
                if let Some(entry) = PoolOps::get(pid) {
                    if entry.status.is_failed() {
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

    let entry = PoolOps::get(pid).ok_or(PoolPolicyError::NotReadyForExport)?;

    policy::pool::export::can_export(&entry)?;

    let role = entry.role.ok_or(PoolPolicyError::MissingRole)?;
    let hash = entry
        .module_hash
        .ok_or(PoolPolicyError::MissingModuleHash)?;

    let _ = PoolOps::take(&pid);

    Ok((role, hash))
}

// -----------------------------------------------------------------------------
// Stats
// -----------------------------------------------------------------------------

#[must_use]
pub fn pool_status_counts() -> PoolStatusCounts {
    let mut counts = PoolStatusCounts::default();

    for (_, entry) in PoolOps::export() {
        match entry.status {
            CanisterPoolStatus::Ready => counts.ready += 1,
            CanisterPoolStatus::PendingReset => counts.pending_reset += 1,
            CanisterPoolStatus::Failed { .. } => counts.failed += 1,
        }
    }

    counts.total = counts.ready + counts.pending_reset + counts.failed;
    counts
}
