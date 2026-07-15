//! Module: workflow::pool::import
//!
//! Responsibility: import external or queued canisters into the reset pool.
//! Does not own: endpoint authorization, stable pool schemas, or pool policy rules.
//! Boundary: workflow helper coordinating admission checks, intents, reset, storage, and metrics.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    domain::pool::CanisterPoolStatus,
    dto::pool::PoolBatchResult,
    ids::{IntentId, IntentResourceKey},
    log,
    log::Topic,
    ops::{
        ic::IcOps,
        runtime::metrics::{
            intent::{
                IntentMetricOperation, IntentMetricOutcome, IntentMetricReason,
                IntentMetricSurface, IntentMetrics,
            },
            pool::{
                PoolMetricOperation as MetricOperation, PoolMetricOutcome as MetricOutcome,
                PoolMetricReason as MetricReason,
            },
            recording::PoolMetricEvent as MetricEvent,
        },
        storage::{intent::IntentStoreOps, pool::PoolOps, registry::subnet::SubnetRegistryOps},
    },
    workflow::{
        pool::{
            PoolWorkflow, admissibility, metric_reason_for_policy, query::PoolQuery,
            scheduler::PoolSchedulerWorkflow,
        },
        runtime::intent::IntentCleanupWorkflow,
    },
};

impl PoolWorkflow {
    pub async fn pool_import_canister(pid: Principal) -> Result<(), InternalError> {
        MetricEvent::started(MetricOperation::ImportImmediate);
        if let Err(err) = Self::require_pool_admin() {
            MetricEvent::failed(MetricOperation::ImportImmediate, &err);
            return Err(err);
        }
        if pool_import_already_present(pid) {
            MetricEvent::skipped(
                MetricOperation::ImportImmediate,
                MetricReason::AlreadyPresent,
            );
            return Ok(());
        }
        if let Err(err) = admissibility::check_can_enter_pool(pid).await {
            MetricEvent::record(
                MetricOperation::ImportImmediate,
                MetricOutcome::Failed,
                metric_reason_for_policy(&err),
            );
            return Err(err.into());
        }

        let intent_key = match pool_import_intent_key(pid) {
            Ok(intent_key) => intent_key,
            Err(err) => {
                MetricEvent::failed(MetricOperation::ImportImmediate, &err);
                return Err(err);
            }
        };

        let intent_id = match reserve_pool_import_intent(intent_key) {
            Ok(intent_id) => intent_id,
            Err(err) => {
                MetricEvent::failed(MetricOperation::ImportImmediate, &err);
                return Err(err);
            }
        };

        // Invariant: mark_pending_reset must remain synchronous and non-trapping.
        Self::mark_pending_reset(pid);

        match Self::reset_into_pool(pid).await {
            Ok(cycles) => {
                let _ = SubnetRegistryOps::unregister(&pid);
                Self::mark_ready(pid, cycles);

                if let Err(err) = commit_pool_import_intent(intent_id, pid) {
                    MetricEvent::failed(MetricOperation::ImportImmediate, &err);
                    return Err(err);
                }

                MetricEvent::completed(MetricOperation::ImportImmediate, MetricReason::Ok);
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

                abort_pool_import_intent(intent_id, pid);

                MetricEvent::failed(MetricOperation::ImportImmediate, &err);
                Err(err)
            }
        }
    }

    pub async fn pool_import_queued_canisters(
        pids: Vec<Principal>,
    ) -> Result<PoolBatchResult, InternalError> {
        MetricEvent::started(MetricOperation::ImportQueued);
        if let Err(err) = Self::require_pool_admin() {
            MetricEvent::failed(MetricOperation::ImportQueued, &err);
            return Err(err);
        }

        Self::pool_import_queued_canisters_authorized(pids, true, true, true, None).await
    }

    async fn pool_import_queued_canisters_authorized(
        pids: Vec<Principal>,
        check_admissibility: bool,
        record_metrics: bool,
        schedule: bool,
        created_at_override: Option<u64>,
    ) -> Result<PoolBatchResult, InternalError> {
        let total = pids.len() as u64;

        let mut added = 0;
        let mut requeued = 0;
        let mut skipped = 0;

        for pid in pids {
            let admission = if check_admissibility {
                admissibility::check_can_enter_pool(pid).await
            } else {
                Ok(())
            };
            match admission {
                Ok(()) => {
                    if let Some(entry) = PoolQuery::pool_entry(pid) {
                        if let CanisterPoolStatus::Failed { .. } = entry.status {
                            mark_pool_import_queued_pending_reset(pid, created_at_override);
                            if record_metrics {
                                MetricEvent::record(
                                    MetricOperation::ImportQueued,
                                    MetricOutcome::Requeued,
                                    MetricReason::FailedEntry,
                                );
                            }
                            requeued += 1;
                        } else {
                            // Already ready or pending reset.
                            if record_metrics {
                                MetricEvent::skipped(
                                    MetricOperation::ImportQueued,
                                    MetricReason::AlreadyPresent,
                                );
                            }
                            skipped += 1;
                        }
                    } else {
                        mark_pool_import_queued_pending_reset(pid, created_at_override);
                        if record_metrics {
                            MetricEvent::completed(MetricOperation::ImportQueued, MetricReason::Ok);
                        }
                        added += 1;
                    }
                }

                Err(err) => {
                    if record_metrics {
                        MetricEvent::record(
                            MetricOperation::ImportQueued,
                            MetricOutcome::Skipped,
                            metric_reason_for_policy(&err),
                        );
                    }
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

        if schedule && (result.added > 0 || result.requeued > 0) {
            PoolSchedulerWorkflow::schedule();
        }

        if record_metrics {
            MetricEvent::completed(MetricOperation::ImportQueued, MetricReason::Ok);
        }

        Ok(result)
    }
}

fn pool_import_already_present(pid: Principal) -> bool {
    PoolOps::contains(&pid)
}

fn mark_pool_import_queued_pending_reset(pid: Principal, created_at_override: Option<u64>) {
    match created_at_override {
        Some(created_at) => PoolOps::mark_pending_reset(pid, created_at),
        None => PoolWorkflow::mark_pending_reset(pid),
    }
}

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

// Reserve the import intent before resetting an external canister into the pool.
fn reserve_pool_import_intent(intent_key: IntentResourceKey) -> Result<IntentId, InternalError> {
    let intent_id = match IntentStoreOps::allocate_intent_id() {
        Ok(intent_id) => intent_id,
        Err(err) => {
            record_pool_intent(
                IntentMetricOperation::Reserve,
                IntentMetricOutcome::Failed,
                IntentMetricReason::StorageFailed,
            );
            return Err(err);
        }
    };

    let now_secs = IcOps::now_secs();
    IntentCleanupWorkflow::ensure_started();
    if let Err(err) =
        IntentStoreOps::try_reserve(intent_id, intent_key, 1, now_secs, None, now_secs)
    {
        record_pool_intent(
            IntentMetricOperation::Reserve,
            IntentMetricOutcome::Failed,
            IntentMetricReason::StorageFailed,
        );
        return Err(err);
    }

    record_pool_intent(
        IntentMetricOperation::Reserve,
        IntentMetricOutcome::Completed,
        IntentMetricReason::Ok,
    );

    Ok(intent_id)
}

// Commit the import intent after the canister has been reset and registered.
fn commit_pool_import_intent(intent_id: IntentId, pid: Principal) -> Result<(), InternalError> {
    if let Err(err) = IntentStoreOps::commit_at(intent_id, IcOps::now_secs()) {
        record_pool_intent(
            IntentMetricOperation::Commit,
            IntentMetricOutcome::Failed,
            IntentMetricReason::StorageFailed,
        );
        log!(
            Topic::CanisterPool,
            Warn,
            "pool import commit failed for {pid}: {err}"
        );
        return Err(err);
    }

    record_pool_intent(
        IntentMetricOperation::Commit,
        IntentMetricOutcome::Completed,
        IntentMetricReason::Ok,
    );
    Ok(())
}

// Abort the import intent after reset fails; the reset error remains authoritative.
fn abort_pool_import_intent(intent_id: IntentId, pid: Principal) {
    if let Err(abort_err) = IntentStoreOps::abort(intent_id) {
        record_pool_intent(
            IntentMetricOperation::Abort,
            IntentMetricOutcome::Failed,
            IntentMetricReason::StorageFailed,
        );
        log!(
            Topic::CanisterPool,
            Warn,
            "pool import abort failed for {pid}: {abort_err}"
        );
    } else {
        record_pool_intent(
            IntentMetricOperation::Abort,
            IntentMetricOutcome::Completed,
            IntentMetricReason::Ok,
        );
    }
}

// Record a pool-surface intent metric with fixed labels only.
fn record_pool_intent(
    operation: IntentMetricOperation,
    outcome: IntentMetricOutcome,
    reason: IntentMetricReason,
) {
    IntentMetrics::record(IntentMetricSurface::Pool, operation, outcome, reason);
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

#[cfg(test)]
mod tests {
    use crate::cdk::types::Cycles;

    use super::*;
    use futures::executor::block_on;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn pool_import_immediate_detects_ready_canister_before_reset() {
        let pid = p(47);
        PoolOps::remove(&pid);

        assert!(!pool_import_already_present(pid));

        PoolOps::register_ready(pid, Cycles::new(10), None, None, None, 100);

        assert!(pool_import_already_present(pid));

        PoolOps::remove(&pid);
    }

    #[test]
    fn pool_import_immediate_detects_pending_reset_canister_before_reset() {
        let pid = p(49);
        PoolOps::remove(&pid);

        assert!(!pool_import_already_present(pid));

        PoolOps::mark_pending_reset(pid, 100);

        assert!(pool_import_already_present(pid));
        assert_eq!(
            PoolQuery::pool_list()
                .entries
                .iter()
                .filter(|entry| entry.pid == pid)
                .count(),
            1,
            "duplicate immediate import must not create another pending entry"
        );
        assert_eq!(
            PoolQuery::pool_entry(pid).expect("pending entry").status,
            CanisterPoolStatus::PendingReset
        );

        PoolOps::remove(&pid);
    }

    #[test]
    fn pool_import_queued_repeated_call_converges_without_duplicate_entries() {
        let pid = p(48);
        PoolOps::remove(&pid);

        let first = block_on(PoolWorkflow::pool_import_queued_canisters_authorized(
            vec![pid, pid],
            false,
            false,
            false,
            Some(100),
        ))
        .expect("first queued import");

        assert_eq!(first.total, 2);
        assert_eq!(first.added, 1);
        assert_eq!(first.requeued, 0);
        assert_eq!(first.skipped, 1);

        let entry = PoolQuery::pool_entry(pid).expect("entry queued");
        assert_eq!(entry.status, CanisterPoolStatus::PendingReset);
        assert_eq!(
            PoolQuery::pool_list()
                .entries
                .iter()
                .filter(|entry| entry.pid == pid)
                .count(),
            1,
            "queued import must not duplicate pool entries"
        );

        let second = block_on(PoolWorkflow::pool_import_queued_canisters_authorized(
            vec![pid, pid],
            false,
            false,
            false,
            Some(100),
        ))
        .expect("second queued import");

        assert_eq!(second.total, 2);
        assert_eq!(second.added, 0);
        assert_eq!(second.requeued, 0);
        assert_eq!(second.skipped, 2);
        assert_eq!(
            PoolQuery::pool_list()
                .entries
                .iter()
                .filter(|entry| entry.pid == pid)
                .count(),
            1,
            "repeated queued import must remain convergent"
        );

        PoolOps::remove(&pid);
    }
}
