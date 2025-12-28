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
    log::Topic,
    ops::{
        OPS_POOL_CHECK_INTERVAL, OPS_POOL_INIT_DELAY,
        ic::timer::{TimerId, TimerOps},
        prelude::*,
        storage::pool::PoolOps,
    },
    policy::pool::PoolPolicyError,
    workflow::pool::{admissibility::can_enter_pool, mark_failed, reset_into_pool},
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
        OPS_POOL_INIT_DELAY,
        "pool:init",
        || async {
            schedule();
        },
        OPS_POOL_CHECK_INTERVAL,
        "pool:interval",
        || async {
            schedule();
        },
    );
}

/// Stop pool background scheduling.
pub fn stop() {
    let _ = TimerOps::clear_guarded(&TIMER);
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
    let mut pending: Vec<_> = PoolOps::export()
        .into_iter()
        .filter(|(_, e)| e.status.is_pending_reset())
        .collect();

    if pending.is_empty() {
        return Ok(());
    }

    pending.sort_by_key(|(_, e)| e.created_at);

    for (pid, _) in pending.into_iter().take(limit) {
        match can_enter_pool(pid).await {
            Ok(()) => {
                // admissible, proceed
            }

            Err(PoolPolicyError::RegisteredInSubnet(_)) => {
                // Permanently forbidden → remove from pool
                let _ = PoolOps::take(&pid);
                continue;
            }

            Err(PoolPolicyError::NonImportableOnLocal { .. }) => {
                // Temporarily not admissible → skip, leave pending
                continue;
            }

            Err(_) => {
                // Defensive: unknown policy failure
                continue;
            }
        }
        match reset_into_pool(pid).await {
            Ok(cycles) => {
                crate::workflow::pool::mark_ready(pid, cycles);
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
    PoolOps::export()
        .into_iter()
        .any(|(_, e)| e.status.is_pending_reset())
}

//
// TEST HOOKS
//

#[cfg(test)]
thread_local! {
    static RESET_SCHEDULED: RefCell<bool> = const { RefCell::new(false) };
}

#[cfg(test)]
pub fn mark_scheduled_for_test() {
    RESET_SCHEDULED.with_borrow_mut(|flag| *flag = true);
}

#[cfg(test)]
#[must_use]
pub fn take_scheduled_for_test() -> bool {
    RESET_SCHEDULED.with_borrow_mut(|flag| {
        let value = *flag;
        *flag = false;
        value
    })
}
