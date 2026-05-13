pub mod query;

use crate::{
    domain::policy,
    ops::{
        config::ConfigOps,
        ic::{IcOps, mgmt::MgmtOps},
        rpc::request::RequestOps,
        runtime::{env::EnvOps, metrics::cycles_topup::CyclesTopupMetrics, timer::TimerId},
        storage::cycles::{CycleTopupEventOps, CycleTrackerOps},
    },
    workflow::{
        config::{WORKFLOW_CYCLE_TRACK_INTERVAL, WORKFLOW_INIT_DELAY},
        prelude::*,
        runtime::timer::TimerWorkflow,
    },
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
    static TOPUP_IN_FLIGHT: RefCell<bool> = const { RefCell::new(false) };
}

const TRACKER_INTERVAL: Duration = WORKFLOW_CYCLE_TRACK_INTERVAL;

///
/// CycleTrackingMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CycleTrackingMode {
    StandardOnly,
    StandardWithAutoTopup,
}

impl CycleTrackingMode {
    const fn auto_topup_enabled(self) -> bool {
        matches!(self, Self::StandardWithAutoTopup)
    }
}

///
/// CycleBalanceSample
///

struct CycleBalanceSample {
    timestamp_secs: u64,
    cycles: Cycles,
}

///
/// CycleTrackerWorkflow
///

pub struct CycleTrackerWorkflow;

impl CycleTrackerWorkflow {
    /// Start recurring cycle tracking.
    /// Safe to call multiple times.
    pub fn start() {
        Self::start_internal(CycleTrackingMode::StandardWithAutoTopup);
    }

    /// Record recurring cycle balance snapshots without top-up decisions.
    /// Safe to call multiple times.
    pub(crate) fn start_standard_only() {
        Self::start_internal(CycleTrackingMode::StandardOnly);
    }

    // Start the recurring cycle tracker with the requested policy surface.
    fn start_internal(mode: CycleTrackingMode) {
        let scheduled = match mode {
            CycleTrackingMode::StandardOnly => Self::schedule_standard_interval(mode),
            CycleTrackingMode::StandardWithAutoTopup => Self::schedule_topup_interval(mode),
        };

        if scheduled {
            Self::record_standard_sample();
        }
    }

    fn schedule_standard_interval(mode: CycleTrackingMode) -> bool {
        TimerWorkflow::set_guarded_interval(
            &TIMER,
            TRACKER_INTERVAL,
            "cycles:interval:first",
            move || async move {
                Self::track_internal(mode);
                let _ = Self::purge();
            },
            TRACKER_INTERVAL,
            "cycles:interval",
            move || async move {
                Self::track_internal(mode);
                let _ = Self::purge();
            },
        )
    }

    fn schedule_topup_interval(mode: CycleTrackingMode) -> bool {
        TimerWorkflow::set_guarded_interval(
            &TIMER,
            WORKFLOW_INIT_DELAY,
            "cycles:topup:first",
            move || async move {
                Self::evaluate_current_topup();
            },
            TRACKER_INTERVAL,
            "cycles:interval",
            move || async move {
                Self::track_internal(mode);
                let _ = Self::purge();
            },
        )
    }

    // Record cycle balance and optionally evaluate auto-top-up policy.
    fn track_internal(mode: CycleTrackingMode) {
        let sample = Self::read_standard_sample();

        if mode.auto_topup_enabled() && !EnvOps::is_root() {
            Self::evaluate_policies(sample.cycles.clone());
        }

        CycleTrackerOps::record(sample.timestamp_secs, sample.cycles);
    }

    fn record_standard_sample() {
        let sample = Self::read_standard_sample();
        CycleTrackerOps::record(sample.timestamp_secs, sample.cycles);
    }

    fn read_standard_sample() -> CycleBalanceSample {
        CycleBalanceSample {
            timestamp_secs: IcOps::now_secs(),
            cycles: MgmtOps::canister_cycle_balance(),
        }
    }

    fn evaluate_current_topup() {
        if EnvOps::is_root() {
            return;
        }

        Self::evaluate_policies(MgmtOps::canister_cycle_balance());
    }

    fn evaluate_policies(cycles: Cycles) {
        Self::check_auto_topup(cycles);
    }

    fn check_auto_topup(cycles: Cycles) {
        let canister_cfg = match ConfigOps::current_canister() {
            Ok(cfg) => cfg,
            Err(err) => {
                CyclesTopupMetrics::record_config_error();
                log!(Topic::Cycles, Warn, "auto topup skipped: {err}");
                return;
            }
        };
        if canister_cfg.topup.is_none() {
            CyclesTopupMetrics::record_policy_missing();
            return;
        }

        let Some(plan) = policy::cycles::should_topup(cycles.to_u128(), &canister_cfg) else {
            CyclesTopupMetrics::record_above_threshold();
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
            CyclesTopupMetrics::record_request_in_flight();
            return;
        }

        CyclesTopupMetrics::record_request_scheduled();
        let requested_cycles = plan.amount;
        CycleTopupEventOps::record_scheduled(IcOps::now_secs(), requested_cycles.clone());
        IcOps::spawn(async move {
            let result = RequestOps::request_cycles(requested_cycles.to_u128()).await;

            TOPUP_IN_FLIGHT.with_borrow_mut(|in_flight| {
                *in_flight = false;
            });

            match result {
                Ok(res) => {
                    CyclesTopupMetrics::record_request_ok();
                    CycleTopupEventOps::record_ok(
                        IcOps::now_secs(),
                        requested_cycles.clone(),
                        Cycles::from(res.cycles_transferred),
                    );
                    log!(
                        Topic::Cycles,
                        Ok,
                        "requested {}, topped up by {}, now {}",
                        requested_cycles,
                        Cycles::from(res.cycles_transferred),
                        MgmtOps::canister_cycle_balance()
                    );
                }
                Err(e) => {
                    CyclesTopupMetrics::record_request_err();
                    CycleTopupEventOps::record_err(
                        IcOps::now_secs(),
                        requested_cycles,
                        e.to_string(),
                    );
                    log!(Topic::Cycles, Error, "failed to request cycles: {e}");
                }
            }
        });
    }

    /// Purge old entries based on the retention window.
    #[must_use]
    pub fn purge() -> bool {
        let now = IcOps::now_secs();
        let cutoff = policy::cycles::retention_cutoff(now);
        let purged = CycleTrackerOps::purge_before(cutoff);
        let purged_topups = CycleTopupEventOps::purge_before(cutoff);

        if purged > 0 || purged_topups > 0 {
            log!(
                Topic::Cycles,
                Info,
                "cycle_tracker: purged {purged} balance entries and {purged_topups} topup events"
            );
        }

        purged > 0
    }
}
