use crate::{
    cdk::{
        futures::spawn,
        timers::{TimerId, clear_timer, set_timer, set_timer_interval},
    },
    interface::ic::canister_cycle_balance,
    log,
    log::Level,
    memory::ext::cycles::{CycleTracker, CycleTrackerView},
    ops::context::cfg_current_canister,
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

/// Wait 10 seconds till we start so the auto-create finishes
const TRACKER_INIT_DELAY: Duration = Duration::new(10, 0);

// Check every 10 mintues
const TRACKER_INTERVAL_SECS: Duration = Duration::from_secs(60 * 10);

///
/// CycleTrackerOps
///

#[derive(CandidType, Serialize)]
pub struct CycleTrackerPage {
    pub entries: CycleTrackerView,
    pub total: u64,
}

pub struct CycleTrackerOps;

impl CycleTrackerOps {
    /// Start recurring tracking every X seconds
    /// Safe to call multiple times: only one loop will run.
    pub fn start() {
        TIMER.with_borrow_mut(|slot| {
            if slot.is_some() {
                return;
            }

            let init = set_timer(TRACKER_INIT_DELAY, async {
                let _ = Self::track();

                let interval = set_timer_interval(TRACKER_INTERVAL_SECS, || async {
                    let _ = Self::track();
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
                clear_timer(id);
            }
        });
    }

    #[must_use]
    pub fn track() -> bool {
        let ts = now_secs();
        let cycles = canister_cycle_balance().to_u128();

        Self::check_auto_topup();

        CycleTracker::record(ts, cycles)
    }

    fn check_auto_topup() {
        use crate::ops::request::cycles_request;

        if let Ok(canister_cfg) = cfg_current_canister()
            && let Some(topup) = canister_cfg.topup
        {
            let cycles = canister_cycle_balance();

            if cycles < topup.threshold {
                spawn(async move {
                    match cycles_request(topup.amount.to_u128()).await {
                        Ok(res) => log!(
                            Level::Ok,
                            "💫 requested {}, topped up by {}, now {}",
                            topup.amount,
                            res.cycles_transferred,
                            canister_cycle_balance()
                        ),
                        Err(e) => log!(Level::Error, "💫 failed to request cycles: {e}"),
                    }
                });
            }
        }
    }

    #[must_use]
    pub fn page(offset: u64, limit: u64) -> CycleTrackerPage {
        let entries = CycleTracker::entries(offset, limit);
        let total = CycleTracker::len();

        CycleTrackerPage { entries, total }
    }
}
