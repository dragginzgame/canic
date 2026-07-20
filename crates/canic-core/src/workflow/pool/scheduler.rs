//! Module: workflow::pool::scheduler
//!
//! Responsibility: schedule and execute background pool reset batches.
//! Does not own: pool admission policy, admin APIs, or stable pool schemas.
//! Boundary: workflow runtime helper coordinating timers, admission checks, reset, and metrics.

use crate::{
    InternalError,
    domain::{policy::pure::pool::PoolPolicyError, runtime::TimerExecutionOutcome},
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
        pool::{PoolWorkflow, admissibility::check_can_enter_pool},
        runtime::timer::{TimerDirective, TimerKey, TimerRunResult, TimerWorkflow},
    },
};
use std::{cell::Cell, time::Duration};

/// Default batch size for resetting pending pool entries.
pub const POOL_RESET_BATCH_SIZE: usize = 10;
const BLOCKED_RETRY_INITIAL: Duration = Duration::from_mins(1);
const BLOCKED_RETRY_MAX: Duration = Duration::from_mins(30);

// -----------------------------------------------------------------------------
// Internal Scheduler State
// -----------------------------------------------------------------------------

thread_local! {
    static RESET_CURSOR: Cell<Option<PoolPendingResetCursor>> = const { Cell::new(None) };
    static SWEEP_MADE_PROGRESS: Cell<bool> = const { Cell::new(false) };
    static SWEEP_RETRYABLE_BLOCKED: Cell<bool> = const { Cell::new(false) };
    static BLOCKED_RETRY_STREAK: Cell<u8> = const { Cell::new(0) };
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct PoolResetBatchResult {
    next_cursor: Option<PoolPendingResetCursor>,
    work_count: u64,
    made_progress: bool,
    retryable_blocked: bool,
}

///
/// PoolSchedulerWorkflow
///

pub struct PoolSchedulerWorkflow;

impl PoolSchedulerWorkflow {
    /// Reconstruct pool scheduling from durable pending-reset state.
    ///
    /// Safe to call multiple times.
    pub fn start() {
        Self::schedule_if_pending();
    }

    /// Schedule a reset worker only when pending reset work exists.
    pub fn schedule_if_pending() {
        if Self::has_pending_reset() {
            Self::schedule();
        } else {
            MetricEvent::skipped(MetricOperation::Scheduler, MetricReason::Empty);
            TimerWorkflow::reconcile_at(TimerKey::PoolReset, None, || async {
                Self::run_scheduled().await
            });
        }
    }

    /// Schedule a reset worker run.
    ///
    /// This is idempotent and guarded against concurrent execution.
    pub fn schedule() {
        BLOCKED_RETRY_STREAK.set(0);
        MetricEvent::record(
            MetricOperation::Scheduler,
            MetricOutcome::Scheduled,
            MetricReason::Ok,
        );
        TimerWorkflow::schedule(TimerKey::PoolReset, Duration::ZERO, || async {
            Self::run_scheduled().await
        });
    }

    async fn run_scheduled() -> TimerRunResult {
        if RESET_CURSOR.get().is_none() {
            SWEEP_MADE_PROGRESS.set(false);
            SWEEP_RETRYABLE_BLOCKED.set(false);
        }

        let Ok(batch) = Self::run_worker(POOL_RESET_BATCH_SIZE).await else {
            RESET_CURSOR.set(None);
            return TimerRunResult::invariant_failure();
        };

        SWEEP_MADE_PROGRESS.set(SWEEP_MADE_PROGRESS.get() || batch.made_progress);
        SWEEP_RETRYABLE_BLOCKED.set(SWEEP_RETRYABLE_BLOCKED.get() || batch.retryable_blocked);
        RESET_CURSOR.set(batch.next_cursor);

        if batch.next_cursor.is_some() {
            return TimerRunResult::success(batch.work_count, TimerDirective::ContinueImmediately);
        }

        if !Self::has_pending_reset() {
            BLOCKED_RETRY_STREAK.set(0);
            return if batch.work_count == 0 {
                TimerRunResult::no_work(TimerDirective::Stop)
            } else {
                TimerRunResult::success(batch.work_count, TimerDirective::Stop)
            };
        }

        if SWEEP_RETRYABLE_BLOCKED.get() {
            if SWEEP_MADE_PROGRESS.get() {
                BLOCKED_RETRY_STREAK.set(0);
            }
            let streak = BLOCKED_RETRY_STREAK.get();
            let delay = blocked_retry_delay(streak);
            BLOCKED_RETRY_STREAK.set(streak.saturating_add(1));
            return TimerRunResult {
                outcome: TimerExecutionOutcome::RetryableFailure,
                work_count: batch.work_count,
                directive: TimerDirective::RetryAfter(delay),
            };
        }

        // A producer may add durable work while the final page is running.
        // Producers also request an immediate run, and this directive makes the
        // recovery independent of callback/request ordering.
        TimerRunResult::success(batch.work_count, TimerDirective::ContinueImmediately)
    }

    async fn run_worker(limit: usize) -> Result<PoolResetBatchResult, InternalError> {
        if limit == 0 {
            MetricEvent::skipped(MetricOperation::Scheduler, MetricReason::Empty);
            return Ok(PoolResetBatchResult::default());
        }

        MetricEvent::started(MetricOperation::Scheduler);
        let result = Self::run_batch(limit).await;
        match &result {
            Ok(_) => MetricEvent::completed(MetricOperation::Scheduler, MetricReason::Ok),
            Err(err) => {
                MetricEvent::failed(MetricOperation::Scheduler, err);
                log!(
                    Topic::CanisterPool,
                    Warn,
                    "pool reset scheduler stopped after invariant failure: {err}"
                );
            }
        }

        result
    }

    async fn run_batch(limit: usize) -> Result<PoolResetBatchResult, InternalError> {
        let after = RESET_CURSOR.get();
        let page = PoolOps::pending_reset_page(after.as_ref(), limit);
        let mut result = PoolResetBatchResult {
            next_cursor: page.next_cursor,
            ..PoolResetBatchResult::default()
        };

        for pid in page.pids {
            result.work_count = result.work_count.saturating_add(1);
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
                    result.made_progress = true;
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
                    result.retryable_blocked = true;
                    continue;
                }

                Err(err) => {
                    return Err(InternalError::from(err).with_diagnostic_context(format!(
                        "unexpected pool reset admissibility result for {pid}"
                    )));
                }
            }

            match PoolWorkflow::reset_into_pool(pid).await {
                Ok(cycles) => {
                    PoolWorkflow::commit_pending_pool_import_intent(pid)?;
                    PoolWorkflow::mark_ready(pid, cycles);
                    result.made_progress = true;
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
                    result.made_progress = true;
                }
            }
        }

        Ok(result)
    }

    fn has_pending_reset() -> bool {
        PoolOps::has_pending_reset()
    }
}

fn blocked_retry_delay(streak: u8) -> Duration {
    let exponent = u32::from(streak.min(5));
    let multiplier = 1u32 << exponent;
    BLOCKED_RETRY_INITIAL
        .checked_mul(multiplier)
        .unwrap_or(BLOCKED_RETRY_MAX)
        .min(BLOCKED_RETRY_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_retry_backoff_is_bounded_and_deterministic() {
        assert_eq!(blocked_retry_delay(0), Duration::from_mins(1));
        assert_eq!(blocked_retry_delay(1), Duration::from_mins(2));
        assert_eq!(blocked_retry_delay(2), Duration::from_mins(4));
        assert_eq!(blocked_retry_delay(3), Duration::from_mins(8));
        assert_eq!(blocked_retry_delay(4), Duration::from_mins(16));
        assert_eq!(blocked_retry_delay(5), Duration::from_mins(30));
        assert_eq!(blocked_retry_delay(u8::MAX), Duration::from_mins(30));
    }
}
