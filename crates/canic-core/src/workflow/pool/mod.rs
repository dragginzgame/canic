pub mod admin;
pub mod admissibility;
pub mod controllers;
pub mod query;
pub mod scheduler;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::TC,
    domain::policy::pool::authority::require_pool_admin,
    dto::pool::{CanisterPoolStatus, PoolBatchResult},
    ids::IntentResourceKey,
    ops::{
        ic::{
            IcOps,
            mgmt::{CanisterSettings, MgmtOps, UpdateSettingsArgs},
        },
        runtime::env::EnvOps,
        runtime::metrics::pool::{
            PoolMetricOperation as MetricOperation, PoolMetricOutcome as MetricOutcome,
            PoolMetricReason as MetricReason, PoolMetrics,
        },
        storage::{intent::IntentStoreOps, pool::PoolOps, registry::subnet::SubnetRegistryOps},
    },
    workflow::{
        pool::{query::PoolQuery, scheduler::PoolSchedulerWorkflow},
        prelude::*,
        runtime::intent::IntentCleanupWorkflow,
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
        PoolMetrics::record(
            MetricOperation::Reset,
            MetricOutcome::Started,
            MetricReason::Ok,
        );
        let controllers = match Self::pool_controllers() {
            Ok(controllers) => controllers,
            Err(err) => {
                PoolMetrics::record(
                    MetricOperation::Reset,
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                return Err(err);
            }
        };

        if let Err(err) = MgmtOps::update_settings(&UpdateSettingsArgs {
            canister_id: pid,
            settings: CanisterSettings {
                controllers: Some(controllers),
                ..Default::default()
            },
            sender_canister_version: None,
        })
        .await
        {
            PoolMetrics::record(
                MetricOperation::Reset,
                MetricOutcome::Failed,
                MetricReason::from_error(&err),
            );
            return Err(err);
        }

        if let Err(err) = MgmtOps::uninstall_code(pid).await {
            PoolMetrics::record(
                MetricOperation::Reset,
                MetricOutcome::Failed,
                MetricReason::from_error(&err),
            );
            return Err(err);
        }

        match MgmtOps::get_cycles(pid).await {
            Ok(cycles) => {
                PoolMetrics::record(
                    MetricOperation::Reset,
                    MetricOutcome::Completed,
                    MetricReason::Ok,
                );
                Ok(cycles)
            }
            Err(err) => {
                PoolMetrics::record(
                    MetricOperation::Reset,
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                Err(err)
            }
        }
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

    #[must_use]
    pub fn pop_oldest_ready() -> Option<Principal> {
        let pid = PoolOps::pop_oldest_ready_pid();
        PoolMetrics::record(
            MetricOperation::SelectReady,
            if pid.is_some() {
                MetricOutcome::Completed
            } else {
                MetricOutcome::Skipped
            },
            if pid.is_some() {
                MetricReason::Ok
            } else {
                MetricReason::Empty
            },
        );
        pid
    }

    #[must_use]
    pub fn pop_oldest_pending_reset() -> Option<Principal> {
        PoolOps::pop_oldest_pending_reset_pid()
    }

    // -------------------------------------------------------------------------
    // Auth
    // -------------------------------------------------------------------------

    fn require_pool_admin() -> Result<(), InternalError> {
        require_pool_admin(EnvOps::is_root()).map_err(Into::into)
    }

    // -------------------------------------------------------------------------
    // Creation
    // -------------------------------------------------------------------------

    pub async fn pool_create_canister() -> Result<Principal, InternalError> {
        PoolMetrics::record(
            MetricOperation::CreateEmpty,
            MetricOutcome::Started,
            MetricReason::Ok,
        );
        if let Err(err) = Self::require_pool_admin() {
            PoolMetrics::record(
                MetricOperation::CreateEmpty,
                MetricOutcome::Failed,
                MetricReason::from_error(&err),
            );
            return Err(err);
        }

        let cycles = Cycles::new(POOL_CANISTER_CYCLES);
        let controllers = match Self::pool_controllers() {
            Ok(controllers) => controllers,
            Err(err) => {
                PoolMetrics::record(
                    MetricOperation::CreateEmpty,
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                return Err(err);
            }
        };
        let pid = match MgmtOps::create_canister(controllers, cycles.clone()).await {
            Ok(pid) => pid,
            Err(err) => {
                PoolMetrics::record(
                    MetricOperation::CreateEmpty,
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                return Err(err);
            }
        };

        let created_at = IcOps::now_secs();
        PoolOps::register_ready(pid, cycles, None, None, None, created_at);

        PoolMetrics::record(
            MetricOperation::CreateEmpty,
            MetricOutcome::Completed,
            MetricReason::Ok,
        );

        Ok(pid)
    }

    // -------------------------------------------------------------------------
    // Import
    // -------------------------------------------------------------------------

    /// Record one immediate-import pool metric row.
    fn record_import_immediate(outcome: MetricOutcome, reason: MetricReason) {
        PoolMetrics::record(MetricOperation::ImportImmediate, outcome, reason);
    }

    pub async fn pool_import_canister(pid: Principal) -> Result<(), InternalError> {
        Self::record_import_immediate(MetricOutcome::Started, MetricReason::Ok);
        if let Err(err) = Self::require_pool_admin() {
            Self::record_import_immediate(MetricOutcome::Failed, MetricReason::from_error(&err));
            return Err(err);
        }
        if let Err(err) = admissibility::check_can_enter_pool(pid).await {
            Self::record_import_immediate(MetricOutcome::Failed, MetricReason::from_policy(&err));
            return Err(err.into());
        }

        let intent_id = match IntentStoreOps::allocate_intent_id() {
            Ok(intent_id) => intent_id,
            Err(err) => {
                Self::record_import_immediate(
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                return Err(err);
            }
        };
        let intent_key = match pool_import_intent_key(pid) {
            Ok(intent_key) => intent_key,
            Err(err) => {
                Self::record_import_immediate(
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                return Err(err);
            }
        };
        let now_secs = IcOps::now_secs();
        let created_at = now_secs;
        IntentCleanupWorkflow::ensure_started();
        if let Err(err) =
            IntentStoreOps::try_reserve(intent_id, intent_key, 1, created_at, None, now_secs)
        {
            Self::record_import_immediate(MetricOutcome::Failed, MetricReason::from_error(&err));
            return Err(err);
        }

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
                    Self::record_import_immediate(
                        MetricOutcome::Failed,
                        MetricReason::from_error(&err),
                    );
                    return Err(err);
                }

                Self::record_import_immediate(MetricOutcome::Completed, MetricReason::Ok);
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

                Self::record_import_immediate(
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                Err(err)
            }
        }
    }

    // -------------------------------------------------------------------------
    // Recycle
    // -------------------------------------------------------------------------

    pub async fn pool_recycle_canister(pid: Principal) -> Result<(), InternalError> {
        PoolMetrics::record(
            MetricOperation::Recycle,
            MetricOutcome::Started,
            MetricReason::Ok,
        );
        if let Err(err) = Self::require_pool_admin() {
            PoolMetrics::record(
                MetricOperation::Recycle,
                MetricOutcome::Failed,
                MetricReason::from_error(&err),
            );
            return Err(err);
        }

        // Recycling a missing child is an idempotent no-op so stale directory cleanup
        // never depends on the provisional child still existing.
        let Some(entry) = SubnetRegistryOps::get(pid) else {
            PoolMetrics::record(
                MetricOperation::Recycle,
                MetricOutcome::Skipped,
                MetricReason::NotFound,
            );
            return Ok(());
        };

        let role = Some(entry.role.clone());
        let module_hash = entry.module_hash.clone();

        // Destructive reset
        let cycles = match Self::reset_into_pool(pid).await {
            Ok(cycles) => cycles,
            Err(err) => {
                PoolMetrics::record(
                    MetricOperation::Recycle,
                    MetricOutcome::Failed,
                    MetricReason::from_error(&err),
                );
                return Err(err);
            }
        };

        // Remove from topology
        let _ = SubnetRegistryOps::remove(&pid);

        // Register back into pool, preserving metadata
        let created_at = IcOps::now_secs();
        PoolOps::register_ready(pid, cycles, role, None, module_hash, created_at);

        PoolMetrics::record(
            MetricOperation::Recycle,
            MetricOutcome::Completed,
            MetricReason::Ok,
        );

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Bulk import
    // -------------------------------------------------------------------------

    pub async fn pool_import_queued_canisters(
        pids: Vec<Principal>,
    ) -> Result<PoolBatchResult, InternalError> {
        PoolMetrics::record(
            MetricOperation::ImportQueued,
            MetricOutcome::Started,
            MetricReason::Ok,
        );
        if let Err(err) = Self::require_pool_admin() {
            PoolMetrics::record(
                MetricOperation::ImportQueued,
                MetricOutcome::Failed,
                MetricReason::from_error(&err),
            );
            return Err(err);
        }

        let total = pids.len() as u64;

        let mut added = 0;
        let mut requeued = 0;
        let mut skipped = 0;

        for pid in pids {
            match admissibility::check_can_enter_pool(pid).await {
                Ok(()) => {
                    if let Some(entry) = PoolQuery::pool_entry(pid) {
                        if let CanisterPoolStatus::Failed { .. } = entry.status {
                            Self::mark_pending_reset(pid);
                            PoolMetrics::record(
                                MetricOperation::ImportQueued,
                                MetricOutcome::Requeued,
                                MetricReason::FailedEntry,
                            );
                            requeued += 1;
                        } else {
                            // already ready or pending reset
                            PoolMetrics::record(
                                MetricOperation::ImportQueued,
                                MetricOutcome::Skipped,
                                MetricReason::AlreadyPresent,
                            );
                            skipped += 1;
                        }
                    } else {
                        Self::mark_pending_reset(pid);
                        PoolMetrics::record(
                            MetricOperation::ImportQueued,
                            MetricOutcome::Completed,
                            MetricReason::Ok,
                        );
                        added += 1;
                    }
                }

                // Any policy rejection is treated as a skip
                Err(err) => {
                    PoolMetrics::record(
                        MetricOperation::ImportQueued,
                        MetricOutcome::Skipped,
                        MetricReason::from_policy(&err),
                    );
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

        PoolMetrics::record(
            MetricOperation::ImportQueued,
            MetricOutcome::Completed,
            MetricReason::Ok,
        );

        Ok(result)
    }
}

//
// ─────────────────────────────────────────────────────────────
// Intent helpers
// ─────────────────────────────────────────────────────────────
//

// Build the stable intent resource key for an imported pool canister.
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

// Encode raw principal bytes as lowercase hex for intent resource keys.
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);

    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }

    out
}
