use crate::{
    Log,
    cdk::{
        futures::spawn,
        timers::{TimerId, clear_timer, set_timer, set_timer_interval},
    },
    interface::ic::canister_cycle_balance,
    log,
    memory::ext::cycles::CycleTracker,
    ops::context::cfg_current_canister,
    utils::time::now_secs,
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

// constants
const TIMER_INTERVAL_SECS: Duration = Duration::from_secs(60 * 10); // 10 minutes

///
/// CycleTracker
/// ops::level logic
///

impl CycleTracker {
    /// Start recurring tracking every X seconds
    /// Safe to call multiple times: only one loop will run.
    pub fn start() {
        TIMER.with_borrow_mut(|slot| {
            if slot.is_some() {
                return;
            }

            let id = set_timer(crate::CANISTER_INIT_DELAY, || {
                let _ = Self::track();

                let interval_id = set_timer_interval(TIMER_INTERVAL_SECS, || {
                    let _ = Self::track();
                });

                TIMER.with_borrow_mut(|slot| *slot = Some(interval_id));
            });

            *slot = Some(id);
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

        Self::record(ts, cycles)
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
                            Log::Ok,
                            "💫 requested {}, topped up by {}, now {}",
                            topup.amount,
                            res.cycles_transferred,
                            canister_cycle_balance()
                        ),
                        Err(e) => log!(Log::Error, "💫 failed to request cycles: {e}"),
                    }
                });
            }
        }
    }
}
