pub use crate::ops::storage::cycles::CycleTrackerView;

use crate::{
    cdk::{futures::spawn, timers::TimerId, utils::time::now_secs},
    log,
    log::Topic,
    ops::{
        OPS_CYCLE_TRACK_INTERVAL, OPS_INIT_DELAY,
        config::ConfigOps,
        env::EnvOps,
        ic::{canister_cycle_balance, timer::TimerOps},
        storage::cycles::CycleTrackerOps,
    },
    types::Cycles,
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
    static TOPUP_IN_FLIGHT: RefCell<bool> = const { RefCell::new(false) };
}

const TRACKER_INTERVAL: Duration = OPS_CYCLE_TRACK_INTERVAL;

/// Start recurring cycle tracking.
/// Safe to call multiple times.
pub fn start() {
    let _ = TimerOps::set_guarded_interval(
        &TIMER,
        OPS_INIT_DELAY,
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
pub fn stop() {
    let _ = TimerOps::clear_guarded(&TIMER);
}

pub fn track() {
    let ts = now_secs();
    let cycles = canister_cycle_balance().to_u128();

    if !EnvOps::is_root() {
        evaluate_policies(cycles);
    }

    CycleTrackerOps::record(ts, cycles);
}

fn evaluate_policies(cycles: u128) {
    check_auto_topup(cycles);
}

fn check_auto_topup(cycles: u128) {
    use crate::ops::rpc::cycles_request;

    let canister_cfg = ConfigOps::current_canister();
    let Some(topup) = canister_cfg.topup else {
        return;
    };

    if cycles >= topup.threshold.to_u128() {
        return;
    }

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
        let result = cycles_request(topup.amount.to_u128()).await;

        TOPUP_IN_FLIGHT.with_borrow_mut(|in_flight| {
            *in_flight = false;
        });

        match result {
            Ok(res) => log!(
                Topic::Cycles,
                Ok,
                "requested {}, topped up by {}, now {}",
                topup.amount,
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
    let purged = CycleTrackerOps::purge(now);

    if purged > 0 {
        log!(
            Topic::Cycles,
            Info,
            "cycle_tracker: purged {purged} old entries"
        );
    }

    purged > 0
}
