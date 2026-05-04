//! Pool reset scheduler and worker runtime.
//!
//! This module owns:
//! - background scheduling
//! - concurrency guards
//! - batch execution
//! - timer wiring
//!
//! It does NOT:
//! - decide *which* canisters belong in the pool
//! - perform policy checks
//! - expose admin APIs
//!
//! All pool semantics live in `workflow.rs`.

use crate::{
    InternalError,
    domain::policy::pool::PoolPolicyError,
    ops::{
        ic::IcOps,
        runtime::{
            metrics::{
                pool::{
                    PoolMetricOperation as MetricOperation, PoolMetricOutcome as MetricOutcome,
                    PoolMetricReason as MetricReason,
                },
                recording::PoolMetricEvent as MetricEvent,
            },
            timer::TimerId,
        },
        storage::pool::PoolOps,
    },
    workflow::{
        config::{WORKFLOW_POOL_CHECK_INTERVAL, WORKFLOW_POOL_INIT_DELAY},
        pool::{PoolWorkflow, admissibility::check_can_enter_pool},
        prelude::*,
        runtime::timer::TimerWorkflow,
    },
};
use std::{cell::RefCell, time::Duration};

/// Default batch size for resetting pending pool entries.
pub const POOL_RESET_BATCH_SIZE: usize = 10;

//
// TIMER STATE
//

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

//
// INTERNAL SCHEDULER STATE
//

thread_local! {
    static RESET_IN_PROGRESS: RefCell<bool> = const { RefCell::new(false) };
    static RESET_RESCHEDULE: RefCell<bool> = const { RefCell::new(false) };
    static RESET_TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
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
        let _ = TimerWorkflow::set_guarded_interval(
            &TIMER,
            WORKFLOW_POOL_INIT_DELAY,
            "pool:init",
            || async {
                Self::schedule_if_pending();
            },
            WORKFLOW_POOL_CHECK_INTERVAL,
            "pool:interval",
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
        let _ = TimerWorkflow::set_guarded(&RESET_TIMER, Duration::ZERO, "pool:pending", async {
            RESET_TIMER.with_borrow_mut(|slot| *slot = None);
            let _ = Self::run_worker(POOL_RESET_BATCH_SIZE).await;
        });
    }

    fn maybe_reschedule() {
        let reschedule = RESET_RESCHEDULE.with_borrow_mut(|flag| {
            let v = *flag;
            *flag = false;
            v
        });

        if reschedule || Self::has_pending_reset() {
            Self::schedule();
        }
    }

    async fn run_worker(limit: usize) -> Result<(), InternalError> {
        if limit == 0 {
            MetricEvent::skipped(MetricOperation::Scheduler, MetricReason::Empty);
            return Ok(());
        }

        let should_run = RESET_IN_PROGRESS.with_borrow_mut(|flag| {
            if *flag {
                RESET_RESCHEDULE.with_borrow_mut(|r| *r = true);
                false
            } else {
                *flag = true;
                true
            }
        });

        if !should_run {
            MetricEvent::skipped(MetricOperation::Scheduler, MetricReason::InProgress);
            return Ok(());
        }

        MetricEvent::started(MetricOperation::Scheduler);
        let result = Self::run_batch(limit).await;

        RESET_IN_PROGRESS.with_borrow_mut(|flag| *flag = false);
        Self::maybe_reschedule();

        MetricEvent::completed(MetricOperation::Scheduler, MetricReason::Ok);

        result
    }

    async fn run_batch(limit: usize) -> Result<(), InternalError> {
        for _ in 0..limit {
            let Some(pid) = PoolWorkflow::pop_oldest_pending_reset() else {
                break;
            };

            match check_can_enter_pool(pid).await {
                Ok(()) => {}

                Err(PoolPolicyError::RegisteredInSubnet(_)) => {
                    PoolOps::remove(&pid);
                    MetricEvent::skipped(MetricOperation::Reset, MetricReason::RegisteredInSubnet);
                    continue;
                }

                Err(PoolPolicyError::NonImportableOnLocal { .. }) => {
                    // Not admissible yet → requeue
                    let created_at = IcOps::now_secs();
                    PoolOps::mark_pending_reset(pid, created_at);
                    MetricEvent::record(
                        MetricOperation::Reset,
                        MetricOutcome::Requeued,
                        MetricReason::NonImportableLocal,
                    );
                    continue;
                }

                Err(err) => {
                    let created_at = IcOps::now_secs();
                    PoolOps::mark_pending_reset(pid, created_at);
                    MetricEvent::record(
                        MetricOperation::Reset,
                        MetricOutcome::Requeued,
                        MetricReason::from_policy(&err),
                    );
                    continue;
                }
            }

            match PoolWorkflow::reset_into_pool(pid).await {
                Ok(cycles) => {
                    PoolWorkflow::mark_ready(pid, cycles);
                }
                Err(err) => {
                    log!(
                        Topic::CanisterPool,
                        Warn,
                        "pool reset failed for {pid}: {err}"
                    );
                    PoolWorkflow::mark_failed(pid, &err);
                }
            }
        }

        Ok(())
    }

    fn has_pending_reset() -> bool {
        PoolOps::has_pending_reset()
    }
}

//
// TEST HOOKS
//

#[cfg(test)]
thread_local! {
    static RESET_SCHEDULED: RefCell<bool> = const { RefCell::new(false) };
}
