pub use crate::ops::storage::cycles::CycleTrackerView;

use crate::{
    cdk::{futures::spawn, utils::time::now_secs},
    dto::Page,
    log,
    log::Topic,
    ops::{
        OPS_CYCLE_TRACK_INTERVAL, OPS_INIT_DELAY,
        config::ConfigOps,
        ic::{
            canister_cycle_balance,
            timer::{TimerId, TimerOps},
        },
        storage::{cycles::CycleTrackerStorageOps, env::EnvOps},
    },
    types::{Cycles, PageRequest},
};
use std::{cell::RefCell, time::Duration};

//
// TIMER
//

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };

    static TOPUP_IN_FLIGHT: RefCell<bool> = const { RefCell::new(false) };
}

///
/// Constants
///

const TRACKER_INTERVAL: Duration = OPS_CYCLE_TRACK_INTERVAL;

///
/// CycleTrackerOps
///

pub struct CycleTrackerOps;

impl CycleTrackerOps {
    /// Start recurring tracking every X seconds
    /// Safe to call multiple times: only one loop will run.
    pub fn start() {
        TIMER.with_borrow_mut(|slot| {
            if slot.is_some() {
                return;
            }

            let init = TimerOps::set(OPS_INIT_DELAY, "cycles:init", async {
                Self::track();

                let interval =
                    TimerOps::set_interval(TRACKER_INTERVAL, "cycles:interval", || async {
                        Self::track();
                        let _ = Self::purge();
                    });

                TIMER.with_borrow_mut(|slot| *slot = Some(interval));
            });

            *slot = Some(init);
        });
    }

    /// Stop recurring tracking.
    pub fn stop() {
        TIMER.with_borrow_mut(|slot| {
            if let Some(id) = slot.take() {
                TimerOps::clear(id);
            }
        });
    }

    pub fn track() {
        let ts = now_secs();
        let cycles = canister_cycle_balance().to_u128();

        if !EnvOps::is_root() {
            Self::evaluate_policies(cycles);
        }

        CycleTrackerStorageOps::record(ts, cycles);
    }

    fn evaluate_policies(cycles: u128) {
        Self::check_auto_topup(cycles);
    }

    fn check_auto_topup(cycles: u128) {
        use crate::ops::rpc::cycles_request;

        // Read per-canister configuration.
        // If no config or no topup policy is defined, nothing to do.
        let Ok(canister_cfg) = ConfigOps::current_canister() else {
            return;
        };
        let Some(topup) = canister_cfg.topup else {
            return;
        };

        // If current balance is above the configured threshold, do not request cycles.
        if cycles >= topup.threshold.to_u128() {
            return;
        }

        // Prevent concurrent or overlapping top-up requests.
        // This avoids spamming root if multiple ticks fire while a request is in flight.
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

        // Perform the top-up asynchronously.
        // The in-flight flag is cleared regardless of success or failure.
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
        let purged = CycleTrackerStorageOps::purge(now);

        if purged > 0 {
            log!(
                Topic::Cycles,
                Info,
                "cycle_tracker: purged {purged} old entries"
            );
        }

        purged > 0
    }

    #[must_use]
    pub fn page(request: PageRequest) -> Page<(u64, Cycles)> {
        let entries = CycleTrackerStorageOps::entries(request);
        let total = CycleTrackerStorageOps::len();

        Page { entries, total }
    }
}
