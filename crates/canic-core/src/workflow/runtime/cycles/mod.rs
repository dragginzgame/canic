pub mod query;

use crate::{
    domain::policy,
    ops::{
        config::ConfigOps,
        ic::{IcOps, mgmt::MgmtOps},
        rpc::request::RequestOps,
        runtime::{
            env::EnvOps,
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

///
/// CycleTrackerWorkflow
///

pub struct CycleTrackerWorkflow;

impl CycleTrackerWorkflow {
    /// Start recurring cycle tracking.
    /// Safe to call multiple times.
    pub fn start() {
        let _ = TimerOps::set_guarded_interval(
            &TIMER,
            WORKFLOW_INIT_DELAY,
            "cycles:init",
            || async {
                Self::track();
            },
            TRACKER_INTERVAL,
            "cycles:interval",
            || async {
                Self::track();
                let _ = Self::purge();
            },
        );
    }

    /// Stop recurring cycle tracking.
    #[expect(dead_code)]
    pub fn stop() {
        let _ = TimerOps::clear_guarded(&TIMER);
    }

    pub fn track() {
        let ts = IcOps::now_secs();
        let cycles = MgmtOps::canister_cycle_balance();

        if !EnvOps::is_root() {
            Self::evaluate_policies(cycles.clone());
        }

        CycleTrackerOps::record(ts, cycles);
    }

    fn evaluate_policies(cycles: Cycles) {
        Self::check_auto_topup(cycles);
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

        IcOps::spawn(async move {
            let result = RequestOps::request_cycles(plan.amount.to_u128()).await;

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
                    MgmtOps::canister_cycle_balance()
                ),
                Err(e) => log!(Topic::Cycles, Error, "failed to request cycles: {e}"),
            }
        });
    }

    /// Purge old entries based on the retention window.
    #[must_use]
    pub fn purge() -> bool {
        let now = IcOps::now_secs();
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
}
