//! Module: workflow::pool::scheduler
//!
//! Responsibility: schedule and execute background pool reset batches.
//! Does not own: pool admission policy, admin APIs, or stable pool schemas.
//! Boundary: workflow runtime helper coordinating timers, admission checks, reset, and metrics.

use crate::{
    InternalError,
    domain::policy::pure::pool::PoolPolicyError,
    log,
    log::Topic,
    ops::{
        runtime::metrics::{
            pool::{
                PoolMetricOperation as MetricOperation, PoolMetricOutcome as MetricOutcome,
                PoolMetricReason as MetricReason,
            },
            recording::PoolMetricEvent as MetricEvent,
        },
        storage::pool::PoolOps,
    },
    view::pool::PoolPendingResetCursor,
    workflow::{
        config::{WORKFLOW_INIT_DELAY, WORKFLOW_POOL_CHECK_INTERVAL},
        pool::{PoolWorkflow, admissibility::check_can_enter_pool, metric_reason_for_policy},
        runtime::timer::{TimerDirective, TimerKey, TimerRunResult, TimerWorkflow},
    },
};
use std::{cell::Cell, time::Duration};

/// Default batch size for resetting pending pool entries.
pub const POOL_RESET_BATCH_SIZE: usize = 10;

// -----------------------------------------------------------------------------
// Internal Scheduler State
// -----------------------------------------------------------------------------

thread_local! {
    static RESET_CURSOR: Cell<Option<PoolPendingResetCursor>> = const { Cell::new(None) };
}

///
/// PoolSchedulerWorkflow
///

pub struct PoolSchedulerWorkflow;

impl PoolSchedulerWorkflow {
    /// Start pool background scheduling.
    ///
    /// Safe to call multiple times.
    pub fn start() {
        TimerWorkflow::ensure_recurring(
            TimerKey::PoolMaintenance,
            WORKFLOW_INIT_DELAY,
            WORKFLOW_POOL_CHECK_INTERVAL,
            || async {
                Self::schedule_if_pending();
            },
        );
    }

    /// Schedule a reset worker only when pending reset work exists.
    pub fn schedule_if_pending() {
        if Self::has_pending_reset() {
            Self::schedule();
        } else {
            MetricEvent::skipped(MetricOperation::Scheduler, MetricReason::Empty);
        }
    }

    /// Schedule a reset worker run.
    ///
    /// This is idempotent and guarded against concurrent execution.
    pub fn schedule() {
        MetricEvent::record(
            MetricOperation::Scheduler,
            MetricOutcome::Scheduled,
            MetricReason::Ok,
        );
        TimerWorkflow::schedule(TimerKey::PoolReset, Duration::ZERO, || async {
            match Self::run_worker(POOL_RESET_BATCH_SIZE).await {
                Ok(has_more) => TimerRunResult::success(
                    1,
                    if has_more {
                        TimerDirective::ContinueImmediately
                    } else {
                        TimerDirective::Stop
                    },
                ),
                Err(_) => TimerRunResult::retryable_failure(TimerDirective::Stop),
            }
        });
    }

    async fn run_worker(limit: usize) -> Result<bool, InternalError> {
        if limit == 0 {
            MetricEvent::skipped(MetricOperation::Scheduler, MetricReason::Empty);
            return Ok(false);
        }

        MetricEvent::started(MetricOperation::Scheduler);
        let result = Self::run_batch(limit).await;
        let next_cursor = result.as_ref().ok().copied().flatten();
        RESET_CURSOR.set(next_cursor);

        match &result {
            Ok(_) => MetricEvent::completed(MetricOperation::Scheduler, MetricReason::Ok),
            Err(err) => MetricEvent::failed(MetricOperation::Scheduler, err),
        }

        result.map(|next_cursor| next_cursor.is_some())
    }

    async fn run_batch(limit: usize) -> Result<Option<PoolPendingResetCursor>, InternalError> {
        let after = RESET_CURSOR.get();
        let page = PoolOps::pending_reset_page(after.as_ref(), limit);

        for pid in page.pids {
            match check_can_enter_pool(pid).await {
                Ok(()) => {}

                Err(PoolPolicyError::RegisteredInSubnet(_)) => {
                    if let Err(err) = PoolWorkflow::abort_pending_pool_import_intent(pid) {
                        log!(
                            Topic::CanisterPool,
                            Warn,
                            "pool reset rejection could not abort import intent for {pid}: {err}"
                        );
                        return Err(err);
                    }
                    PoolOps::remove(&pid);
                    MetricEvent::skipped(MetricOperation::Reset, MetricReason::RegisteredInSubnet);
                    continue;
                }

                Err(PoolPolicyError::NonImportableOnLocal { .. }) => {
                    // The authoritative pending record remains queued.
                    MetricEvent::record(
                        MetricOperation::Reset,
                        MetricOutcome::Requeued,
                        MetricReason::NonImportableLocal,
                    );
                    continue;
                }

                Err(err) => {
                    MetricEvent::record(
                        MetricOperation::Reset,
                        MetricOutcome::Requeued,
                        metric_reason_for_policy(&err),
                    );
                    continue;
                }
            }

            match PoolWorkflow::reset_into_pool(pid).await {
                Ok(cycles) => {
                    PoolWorkflow::commit_pending_pool_import_intent(pid)?;
                    PoolWorkflow::mark_ready(pid, cycles);
                }
                Err(err) => {
                    log!(
                        Topic::CanisterPool,
                        Warn,
                        "pool reset failed for {pid}: {err}"
                    );
                    if let Err(abort_err) = PoolWorkflow::abort_pending_pool_import_intent(pid) {
                        return Err(err.with_diagnostic_context(format!(
                            "pool import intent abort failed for {pid}: {abort_err}"
                        )));
                    }
                    PoolWorkflow::mark_failed(pid, &err);
                }
            }
        }

        Ok(page.next_cursor)
    }

    fn has_pending_reset() -> bool {
        PoolOps::has_pending_reset()
    }
}
