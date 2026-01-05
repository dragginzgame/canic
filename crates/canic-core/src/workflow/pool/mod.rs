pub mod admin;
pub mod admissibility;
pub mod controllers;
pub mod mapper;
pub mod query;
pub mod scheduler;

use crate::{
    Error,
    access::env,
    domain::policy::pool::PoolPolicyError,
    dto::pool::{CanisterPoolStatusView, PoolBatchResult},
    ops::{
        ic::mgmt::{CanisterSettings, MgmtOps, UpdateSettingsArgs},
        ic::runtime::TC,
        storage::{
            pool::{PoolEntrySnapshot, PoolOps, PoolSnapshot, PoolStatus},
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::{pool::query::pool_entry_view, pool::scheduler::PoolSchedulerWorkflow, prelude::*},
};

/// Default cycles allocated to freshly created pool canisters.
const POOL_CANISTER_CYCLES: u128 = 5 * TC;

///
/// PoolWorkflow
///

pub struct PoolWorkflow;

impl PoolWorkflow {
    // -------------------------------------------------------------------------
    // Reset
    // -------------------------------------------------------------------------

    pub async fn reset_into_pool(pid: Principal) -> Result<Cycles, Error> {
        MgmtOps::update_settings(&UpdateSettingsArgs {
            canister_id: pid,
            settings: CanisterSettings {
                controllers: Some(Self::pool_controllers()?),
                ..Default::default()
            },
            sender_canister_version: None,
        })
        .await?;

        MgmtOps::uninstall_code(pid).await?;
        MgmtOps::get_cycles(pid).await
    }

    // -------------------------------------------------------------------------
    // Metadata helpers
    // -------------------------------------------------------------------------

    fn mark_pending_reset(pid: Principal) {
        PoolOps::mark_pending_reset(pid);
    }

    fn mark_ready(pid: Principal, cycles: Cycles) {
        PoolOps::mark_ready(pid, cycles);
    }

    fn mark_failed(pid: Principal, err: &Error) {
        PoolOps::mark_failed(pid, err);
    }

    // -------------------------------------------------------------------------
    // Selection
    // -------------------------------------------------------------------------

    pub fn pop_oldest_ready() -> Option<PoolEntrySnapshot> {
        Self::pop_oldest_by_status(PoolStatus::Ready)
    }

    pub fn pop_oldest_pending_reset() -> Option<PoolEntrySnapshot> {
        Self::pop_oldest_by_status(PoolStatus::PendingReset)
    }

    fn pop_oldest_by_status(status: PoolStatus) -> Option<PoolEntrySnapshot> {
        let snapshot = PoolOps::snapshot();
        let entry = Self::select_oldest(snapshot, &status)?;
        PoolOps::remove(&entry.pid);

        Some(entry)
    }

    fn select_oldest(snapshot: PoolSnapshot, status: &PoolStatus) -> Option<PoolEntrySnapshot> {
        let mut selected: Option<PoolEntrySnapshot> = None;

        for entry in snapshot.entries {
            let matches = match status {
                PoolStatus::Ready => matches!(entry.status, PoolStatus::Ready),
                PoolStatus::PendingReset => matches!(entry.status, PoolStatus::PendingReset),
                PoolStatus::Failed { .. } => false,
            };

            if !matches {
                continue;
            }

            let replace = match &selected {
                None => true,
                Some(best) => {
                    entry.created_at < best.created_at
                        || (entry.created_at == best.created_at
                            && entry.pid.as_slice() < best.pid.as_slice())
                }
            };

            if replace {
                selected = Some(entry);
            }
        }

        selected
    }

    // -------------------------------------------------------------------------
    // Auth
    // -------------------------------------------------------------------------

    fn require_pool_admin() -> Result<(), Error> {
        env::require_root()?;

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Creation
    // -------------------------------------------------------------------------

    pub async fn pool_create_canister() -> Result<Principal, Error> {
        Self::require_pool_admin()?;

        let cycles = Cycles::new(POOL_CANISTER_CYCLES);
        let pid = MgmtOps::create_canister(Self::pool_controllers()?, cycles.clone()).await?;

        PoolOps::register_ready(pid, cycles, None, None, None);

        Ok(pid)
    }

    // -------------------------------------------------------------------------
    // Import
    // -------------------------------------------------------------------------

    pub async fn pool_import_canister(pid: Principal) -> Result<(), Error> {
        Self::require_pool_admin()?;
        admissibility::check_can_enter_pool(pid).await?;

        Self::mark_pending_reset(pid);

        match Self::reset_into_pool(pid).await {
            Ok(cycles) => {
                let _ = SubnetRegistryOps::remove(&pid);
                Self::mark_ready(pid, cycles);
                Ok(())
            }
            Err(err) => {
                log!(
                    Topic::CanisterPool,
                    Warn,
                    "pool import failed for {pid}: {err}"
                );
                Self::mark_failed(pid, &err);

                Err(err)
            }
        }
    }

    // -------------------------------------------------------------------------
    // Recycle
    // -------------------------------------------------------------------------

    pub async fn pool_recycle_canister(pid: Principal) -> Result<(), Error> {
        Self::require_pool_admin()?;

        // Must exist in registry to be recycled
        let entry =
            SubnetRegistryOps::get(pid).ok_or(PoolPolicyError::NotRegisteredInSubnet(pid))?;

        let role = Some(entry.role.clone());
        let module_hash = entry.module_hash.clone();

        // Destructive reset
        let cycles = Self::reset_into_pool(pid).await?;

        // Remove from topology
        let _ = SubnetRegistryOps::remove(&pid);

        // Register back into pool, preserving metadata
        PoolOps::register_ready(pid, cycles, role, None, module_hash);

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Bulk import
    // -------------------------------------------------------------------------

    pub async fn pool_import_queued_canisters(
        pids: Vec<Principal>,
    ) -> Result<PoolBatchResult, Error> {
        Self::require_pool_admin()?;

        let total = pids.len() as u64;

        let mut added = 0;
        let mut requeued = 0;
        let mut skipped = 0;

        for pid in pids {
            match admissibility::check_can_enter_pool(pid).await {
                Ok(()) => {
                    if let Some(entry) = pool_entry_view(pid) {
                        match entry.status {
                            CanisterPoolStatusView::Failed { .. } => {
                                Self::mark_pending_reset(pid);
                                requeued += 1;
                            }
                            _ => {
                                // already ready or pending reset
                                skipped += 1;
                            }
                        }
                    } else {
                        Self::mark_pending_reset(pid);
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
            PoolSchedulerWorkflow::schedule();
        }

        Ok(result)
    }
}
