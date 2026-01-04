use crate::{
    access::env,
    cdk::{futures::spawn, utils::time::now_secs},
    domain::policy,
    ops::{
        config::ConfigOps,
        ic::mgmt::canister_cycle_balance,
        rpc::request::cycles_request,
        runtime::{
            env as runtime_env,
            timer::{TimerId, TimerOps},
        },
        storage::cycles::CycleTrackerOps,
    },
    workflow::{
        config::{WORKFLOW_CYCLE_TRACK_INTERVAL, WORKFLOW_INIT_DELAY},
        prelude::*,
    },
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
    static TOPUP_IN_FLIGHT: RefCell<bool> = const { RefCell::new(false) };
}

const TRACKER_INTERVAL: Duration = WORKFLOW_CYCLE_TRACK_INTERVAL;

/// Start recurring cycle tracking.
/// Safe to call multiple times.
pub fn start() {
    let _ = TimerOps::set_guarded_interval(
        &TIMER,
        WORKFLOW_INIT_DELAY,
        "cycles:init",
        || async {
            track();
        },
        TRACKER_INTERVAL,
        "cycles:interval",
        || async {
            track();
            let _ = purge();
        },
    );
}

/// Stop recurring cycle tracking.
#[expect(dead_code)]
pub fn stop() {
    let _ = TimerOps::clear_guarded(&TIMER);
}

pub fn track() {
    let ts = now_secs();
    let cycles = canister_cycle_balance();

    if !runtime_env::is_root() {
        evaluate_policies(cycles.clone());
    }

    CycleTrackerOps::record(ts, cycles);
}

fn evaluate_policies(cycles: Cycles) {
    check_auto_topup(cycles);
}

fn check_auto_topup(cycles: Cycles) {
    let canister_cfg = match ConfigOps::current_canister() {
        Ok(cfg) => cfg,
        Err(err) => {
            log!(Topic::Cycles, Warn, "auto topup skipped: {err}");
            return;
        }
    };
    let Some(plan) = policy::cycles::should_topup(cycles.to_u128(), &canister_cfg) else {
        return;
    };

    let should_request = TOPUP_IN_FLIGHT.with_borrow_mut(|in_flight| {
        if *in_flight {
            false
        } else {
            *in_flight = true;
            true
        }
    });

    if !should_request {
        return;
    }

    spawn(async move {
        let result = match env::deny_root() {
            Ok(()) => cycles_request(plan.amount.to_u128()).await,
            Err(err) => Err(err),
        };

        TOPUP_IN_FLIGHT.with_borrow_mut(|in_flight| {
            *in_flight = false;
        });

        match result {
            Ok(res) => log!(
                Topic::Cycles,
                Ok,
                "requested {}, topped up by {}, now {}",
                plan.amount,
                Cycles::from(res.cycles_transferred),
                canister_cycle_balance()
            ),
            Err(e) => log!(Topic::Cycles, Error, "failed to request cycles: {e}"),
        }
    });
}

/// Purge old entries based on the retention window.
#[must_use]
pub fn purge() -> bool {
    let now = now_secs();
    let cutoff = policy::cycles::retention_cutoff(now);
    let purged = CycleTrackerOps::purge_before(cutoff);

    if purged > 0 {
        log!(
            Topic::Cycles,
            Info,
            "cycle_tracker: purged {purged} old entries"
        );
    }

    purged > 0
}
