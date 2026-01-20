pub mod admin;
pub mod admissibility;
pub mod controllers;
pub mod query;
pub mod scheduler;

use crate::{
    InternalError, InternalErrorOrigin,
    domain::policy::pool::PoolPolicyError,
    dto::pool::{CanisterPoolStatus, PoolBatchResult},
    ids::IntentResourceKey,
    ops::{
        ic::{
            IcOps, TC,
            mgmt::{CanisterSettings, MgmtOps, UpdateSettingsArgs},
        },
        runtime::env::EnvOps,
        storage::{intent::IntentStoreOps, pool::PoolOps, registry::subnet::SubnetRegistryOps},
    },
    workflow::{
        pool::{query::PoolQuery, scheduler::PoolSchedulerWorkflow},
        prelude::*,
    },
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

    pub async fn reset_into_pool(pid: Principal) -> Result<Cycles, InternalError> {
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
        let created_at = IcOps::now_secs();
        PoolOps::mark_pending_reset(pid, created_at);
    }

    fn mark_ready(pid: Principal, cycles: Cycles) {
        let created_at = IcOps::now_secs();
        PoolOps::mark_ready(pid, cycles, created_at);
    }

    fn mark_failed(pid: Principal, err: &InternalError) {
        let created_at = IcOps::now_secs();
        PoolOps::mark_failed(pid, err, created_at);
    }

    // -------------------------------------------------------------------------
    // Selection
    // -------------------------------------------------------------------------

    pub fn pop_oldest_ready() -> Option<Principal> {
        PoolOps::pop_oldest_ready_pid()
    }

    pub fn pop_oldest_pending_reset() -> Option<Principal> {
        PoolOps::pop_oldest_pending_reset_pid()
    }

    // -------------------------------------------------------------------------
    // Auth
    // -------------------------------------------------------------------------

    fn require_pool_admin() -> Result<(), InternalError> {
        EnvOps::require_root()?;

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Creation
    // -------------------------------------------------------------------------

    pub async fn pool_create_canister() -> Result<Principal, InternalError> {
        Self::require_pool_admin()?;

        let cycles = Cycles::new(POOL_CANISTER_CYCLES);
        let pid = MgmtOps::create_canister(Self::pool_controllers()?, cycles.clone()).await?;

        let created_at = IcOps::now_secs();
        PoolOps::register_ready(pid, cycles, None, None, None, created_at);

        Ok(pid)
    }

    // -------------------------------------------------------------------------
    // Import
    // -------------------------------------------------------------------------

    pub async fn pool_import_canister(pid: Principal) -> Result<(), InternalError> {
        Self::require_pool_admin()?;
        admissibility::check_can_enter_pool(pid).await?;

        let intent_id = IntentStoreOps::allocate_intent_id()?;
        let intent_key = pool_import_intent_key(pid)?;
        let created_at = IcOps::now_secs();
        let _ = IntentStoreOps::try_reserve(intent_id, intent_key, 1, created_at, None)?;

        // Invariant: mark_pending_reset must remain synchronous and non-trapping.
        Self::mark_pending_reset(pid);

        match Self::reset_into_pool(pid).await {
            Ok(cycles) => {
                let _ = SubnetRegistryOps::remove(&pid);
                Self::mark_ready(pid, cycles);

                if let Err(err) = IntentStoreOps::commit_at(intent_id, IcOps::now_secs()) {
                    log!(
                        Topic::CanisterPool,
                        Warn,
                        "pool import commit failed for {pid}: {err}"
                    );
                    return Err(err);
                }

                Ok(())
            }
            Err(err) => {
                let (class, origin) = err.log_fields();
                log!(
                    Topic::CanisterPool,
                    Warn,
                    "pool import failed for {pid} class={class} origin={origin}: {err}"
                );
                Self::mark_failed(pid, &err);

                if let Err(abort_err) = IntentStoreOps::abort(intent_id) {
                    log!(
                        Topic::CanisterPool,
                        Warn,
                        "pool import abort failed for {pid}: {abort_err}"
                    );
                }

                Err(err)
            }
        }
    }

    // -------------------------------------------------------------------------
    // Recycle
    // -------------------------------------------------------------------------

    pub async fn pool_recycle_canister(pid: Principal) -> Result<(), InternalError> {
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
        let created_at = IcOps::now_secs();
        PoolOps::register_ready(pid, cycles, role, None, module_hash, created_at);

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Bulk import
    // -------------------------------------------------------------------------

    pub async fn pool_import_queued_canisters(
        pids: Vec<Principal>,
    ) -> Result<PoolBatchResult, InternalError> {
        Self::require_pool_admin()?;

        let total = pids.len() as u64;

        let mut added = 0;
        let mut requeued = 0;
        let mut skipped = 0;

        for pid in pids {
            match admissibility::check_can_enter_pool(pid).await {
                Ok(()) => {
                    if let Some(entry) = PoolQuery::pool_entry(pid) {
                        match entry.status {
                            CanisterPoolStatus::Failed { .. } => {
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

//
// ─────────────────────────────────────────────────────────────
// Intent helpers
// ─────────────────────────────────────────────────────────────
//

fn pool_import_intent_key(pid: Principal) -> Result<IntentResourceKey, InternalError> {
    let bytes = pid.as_slice();
    let mut buf = String::with_capacity(3 + bytes.len() * 2);
    buf.push_str("pi:");
    buf.push_str(&hex_encode(bytes));

    IntentResourceKey::try_new(buf).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("pool import intent key: {err}"),
        )
    })
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);

    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }

    out
}
