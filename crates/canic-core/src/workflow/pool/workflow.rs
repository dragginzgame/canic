//! Pool workflow orchestration.

use crate::{
    Error,
    cdk::mgmt::{CanisterSettings, UpdateSettingsArgs},
    log,
    log::Topic,
    ops::{
        ic::{
            get_cycles,
            mgmt::{create_canister, uninstall_code},
            update_settings,
        },
        pool::pool_controllers,
        storage::{pool::CanisterPoolStorageOps, topology::SubnetCanisterRegistryOps},
    },
    policy::pool::{self as policy, PoolPolicyError},
    types::{Cycles, TC},
};
use candid::Principal;

use crate::model::memory::pool::CanisterPoolStatus;

use crate::workflow::pool::dto::{PoolImportSummary, PoolStatusCounts};

/// Default cycles allocated to freshly created pool canisters.
const POOL_CANISTER_CYCLES: u128 = 5 * TC;

//
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

//
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
    if let Some(mut entry) = CanisterPoolStorageOps::get(pid) {
        entry.cycles = cycles;
        entry.status = status;
        entry.role = role.or(entry.role);
        entry.parent = parent.or(entry.parent);
        entry.module_hash = module_hash.or(entry.module_hash);
        let _ = CanisterPoolStorageOps::update(pid, entry);
    } else {
        CanisterPoolStorageOps::register(pid, cycles, status, role, parent, module_hash);
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

//
// -----------------------------------------------------------------------------
// Creation
// -----------------------------------------------------------------------------

pub async fn pool_create_canister() -> Result<Principal, Error> {
    policy::authority::require_pool_admin()?;

    let cycles = Cycles::new(POOL_CANISTER_CYCLES);
    let pid = create_canister(pool_controllers(), cycles.clone()).await?;

    CanisterPoolStorageOps::register(pid, cycles, CanisterPoolStatus::Ready, None, None, None);

    Ok(pid)
}

//
// -----------------------------------------------------------------------------
// Import
// -----------------------------------------------------------------------------

pub async fn pool_import_canister(pid: Principal) -> Result<(), Error> {
    policy::authority::require_pool_admin()?;

    policy::admissibility::assert_can_import(pid)?;

    mark_pending_reset(pid);

    match reset_into_pool(pid).await {
        Ok(cycles) => {
            let _ = SubnetCanisterRegistryOps::remove(&pid);
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

//
// -----------------------------------------------------------------------------
// Bulk import
// -----------------------------------------------------------------------------

pub fn pool_import_queued_canisters(
    pids: Vec<Principal>,
) -> Result<(u64, u64, u64, u64, PoolImportSummary), Error> {
    policy::authority::require_pool_admin()?;

    let mut summary = PoolImportSummary::default();
    let total = pids.len() as u64;

    let mut added = 0;
    let mut requeued = 0;
    let mut skipped = 0;

    for pid in pids {
        match policy::admissibility::check_can_import(pid) {
            Ok(()) => {
                mark_pending_reset(pid);
                added += 1;
            }
            Err(PoolPolicyError::RegisteredInSubnet(_)) => {
                skipped += 1;
                summary.skipped_in_registry += 1;
            }
            Err(_) => {
                skipped += 1;
            }
        }
    }

    summary.status_counts = pool_status_counts();

    Ok((added, requeued, skipped, total, summary))
}

//
// -----------------------------------------------------------------------------
// Export
// -----------------------------------------------------------------------------

pub async fn pool_export_canister(
    pid: Principal,
) -> Result<(crate::ids::CanisterRole, Vec<u8>), Error> {
    policy::authority::require_pool_admin()?;

    let entry = CanisterPoolStorageOps::get(pid).ok_or(PoolPolicyError::NotReadyForExport)?;

    policy::export::assert_exportable(&entry)?;

    let role = entry.role.ok_or(PoolPolicyError::MissingRole)?;
    let hash = entry
        .module_hash
        .ok_or(PoolPolicyError::MissingModuleHash)?;

    let _ = CanisterPoolStorageOps::take(&pid);

    Ok((role, hash))
}

//
// -----------------------------------------------------------------------------
// Stats
// -----------------------------------------------------------------------------

pub fn pool_status_counts() -> PoolStatusCounts {
    let mut counts = PoolStatusCounts::default();

    for (_, entry) in CanisterPoolStorageOps::export() {
        match entry.status {
            CanisterPoolStatus::Ready => counts.ready += 1,
            CanisterPoolStatus::PendingReset => counts.pending_reset += 1,
            CanisterPoolStatus::Failed { .. } => counts.failed += 1,
        }
    }

    counts.total = counts.ready + counts.pending_reset + counts.failed;
    counts
}
