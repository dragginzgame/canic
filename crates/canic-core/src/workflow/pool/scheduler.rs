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
    Error,
    domain::policy::pool::PoolPolicyError,
    ops::{
        runtime::timer::{TimerId, TimerOps},
        storage::pool::PoolOps,
    },
    workflow::{
        config::{WORKFLOW_POOL_CHECK_INTERVAL, WORKFLOW_POOL_INIT_DELAY},
        pool::{
            admissibility::check_can_enter_pool, mark_failed, mark_ready, pop_oldest_pending_reset,
            reset_into_pool,
        },
        prelude::*,
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

/// Start pool background scheduling.
///
/// Safe to call multiple times.
pub fn start() {
    let _ = TimerOps::set_guarded_interval(
        &TIMER,
        WORKFLOW_POOL_INIT_DELAY,
        "pool:init",
        || async {
            schedule();
        },
        WORKFLOW_POOL_CHECK_INTERVAL,
        "pool:interval",
        || async {
            schedule();
        },
    );
}

/// Schedule a reset worker run.
///
/// This is idempotent and guarded against concurrent execution.
pub fn schedule() {
    let _ = TimerOps::set_guarded(&RESET_TIMER, Duration::ZERO, "pool:pending", async {
        RESET_TIMER.with_borrow_mut(|slot| *slot = None);
        let _ = run_worker(POOL_RESET_BATCH_SIZE).await;
    });
}

fn maybe_reschedule() {
    let reschedule = RESET_RESCHEDULE.with_borrow_mut(|flag| {
        let v = *flag;
        *flag = false;
        v
    });

    if reschedule || has_pending_reset() {
        schedule();
    }
}

async fn run_worker(limit: usize) -> Result<(), Error> {
    if limit == 0 {
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
        return Ok(());
    }

    let result = run_batch(limit).await;

    RESET_IN_PROGRESS.with_borrow_mut(|flag| *flag = false);
    maybe_reschedule();

    result
}

async fn run_batch(limit: usize) -> Result<(), Error> {
    for _ in 0..limit {
        let Some(entry) = pop_oldest_pending_reset() else {
            break;
        };

        let pid = entry.pid;

        match check_can_enter_pool(pid).await {
            Ok(()) => {}

            Err(PoolPolicyError::RegisteredInSubnet(_)) => {
                PoolOps::remove(&pid);
                continue;
            }

            Err(PoolPolicyError::NonImportableOnLocal { .. }) => {
                // Not admissible yet → requeue
                PoolOps::mark_pending_reset(pid);
                continue;
            }

            Err(_) => {
                PoolOps::mark_pending_reset(pid);
                continue;
            }
        }

        match reset_into_pool(pid).await {
            Ok(cycles) => {
                mark_ready(pid, cycles);
            }
            Err(err) => {
                log!(
                    Topic::CanisterPool,
                    Warn,
                    "pool reset failed for {pid}: {err}"
                );
                mark_failed(pid, &err);
            }
        }
    }

    Ok(())
}
fn has_pending_reset() -> bool {
    PoolOps::has_pending_reset()
}
//
// TEST HOOKS
//

#[cfg(test)]
thread_local! {
    static RESET_SCHEDULED: RefCell<bool> = const { RefCell::new(false) };
}
