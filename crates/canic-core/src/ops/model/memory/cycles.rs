pub use crate::model::memory::cycles::CycleTrackerView;

use crate::{
    cdk::futures::spawn,
    interface::ic::{
        canister_cycle_balance,
        timer::{Timer, TimerId},
    },
    log,
    log::Topic,
    model::memory::cycles::CycleTracker,
    ops::{
        config::ConfigOps,
        model::memory::EnvOps,
        model::{OPS_CYCLE_TRACK_INTERVAL, OPS_INIT_DELAY},
        timer::TimerOps,
    },
    types::{Cycles, PageRequest},
    utils::time::now_secs,
};
use candid::CandidType;
use serde::Serialize;
use std::{cell::RefCell, time::Duration};

//
// TIMER
//

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

///
/// Constants
///

// Check every 10 minutes
const TRACKER_INTERVAL_SECS: Duration = OPS_CYCLE_TRACK_INTERVAL;

///
/// CycleTrackerPage
///

#[derive(CandidType, Serialize)]
pub struct CycleTrackerPage {
    pub entries: CycleTrackerView,
    pub total: u64,
}

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
                let _ = Self::track();

                let interval =
                    TimerOps::set_interval(TRACKER_INTERVAL_SECS, "cycles:interval", || async {
                        let _ = Self::track();
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
                Timer::clear(id);
            }
        });
    }

    #[must_use]
    pub fn track() -> bool {
        let ts = now_secs();
        let cycles = canister_cycle_balance().to_u128();

        // only check for topup on non-root canisters
        if !EnvOps::is_root() {
            Self::check_auto_topup();
        }

        CycleTracker::record(ts, cycles)
    }

    /// Purge old entries based on the retention window.
    #[must_use]
    pub fn purge() -> bool {
        let now = now_secs();
        CycleTracker::purge(now) > 0
    }

    fn check_auto_topup() {
        use crate::ops::request::cycles_request;

        if let Ok(canister_cfg) = ConfigOps::current_canister()
            && let Some(topup) = canister_cfg.topup
        {
            let cycles = canister_cycle_balance();

            if cycles < topup.threshold {
                spawn(async move {
                    match cycles_request(topup.amount.to_u128()).await {
                        Ok(res) => log!(
                            Topic::Cycles,
                            Ok,
                            "💫 requested {}, topped up by {}, now {}",
                            topup.amount,
                            Cycles::from(res.cycles_transferred),
                            canister_cycle_balance()
                        ),
                        Err(e) => log!(Topic::Cycles, Error, "💫 failed to request cycles: {e}"),
                    }
                });
            }
        }
    }

    #[must_use]
    pub fn page(request: PageRequest) -> CycleTrackerPage {
        let entries = CycleTracker::entries(request);
        let total = CycleTracker::len();

        CycleTrackerPage { entries, total }
    }
}
