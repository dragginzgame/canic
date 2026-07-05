pub mod query;

use crate::{
    domain::policy,
    dto::icp_refill::IcpRefillStatus,
    ops::{
        config::ConfigOps,
        ic::IcOps,
        rpc::request::RequestOps,
        runtime::{env::EnvOps, metrics::cycles_topup::CyclesTopupMetrics, timer::TimerId},
        storage::cycles::{CycleTopupEventOps, CycleTrackerOps},
    },
    workflow::{
        config::{WORKFLOW_CYCLE_TRACK_INTERVAL, WORKFLOW_INIT_DELAY},
        ic::icp_refill::IcpRefillWorkflow,
        prelude::*,
        runtime::timer::TimerWorkflow,
    },
};
use std::{cell::RefCell, thread::LocalKey, time::Duration};

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
    static TOPUP_IN_FLIGHT: RefCell<bool> = const { RefCell::new(false) };
    static ICP_REFILL_IN_FLIGHT: RefCell<bool> = const { RefCell::new(false) };
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

        if mode.auto_topup_enabled() {
            if EnvOps::is_root() {
                if Self::check_hub_self_refill(&sample.cycles) {
                    CycleTrackerOps::record(sample.timestamp_secs, sample.cycles);
                    return;
                }
            } else {
                Self::check_auto_topup(&sample.cycles);
            }
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
            cycles: IcOps::canister_cycle_balance(),
        }
    }

    fn evaluate_current_topup() {
        let cycles = IcOps::canister_cycle_balance();
        if EnvOps::is_root() {
            Self::check_hub_self_refill(&cycles);
            return;
        }

        Self::check_auto_topup(&cycles);
    }

    fn check_hub_self_refill(cycles: &Cycles) -> bool {
        let canister_cfg = match ConfigOps::current_canister() {
            Ok(cfg) => cfg,
            Err(err) => {
                CyclesTopupMetrics::record_config_error();
                log!(Topic::Cycles, Warn, "hub ICP self-refill skipped: {err}");
                return false;
            }
        };
        let Some(topup) = canister_cfg.topup else {
            return false;
        };
        let Some(icp_refill) = topup.icp_refill else {
            return false;
        };
        if !icp_refill.enabled {
            return false;
        }
        if cycles.to_u128() >= icp_refill.min_hub_cycles_before_refill.to_u128() {
            return false;
        }

        let should_refill = mark_in_flight(&ICP_REFILL_IN_FLIGHT);

        if !should_refill {
            CyclesTopupMetrics::record_request_in_flight();
            return true;
        }

        CyclesTopupMetrics::record_request_scheduled();
        let hub_cycles = cycles.clone();
        IcOps::spawn(async move {
            let result = IcpRefillWorkflow::execute_hub_self_refill(hub_cycles).await;

            clear_in_flight(&ICP_REFILL_IN_FLIGHT);

            match result {
                Ok(response) if response.status == IcpRefillStatus::Completed => {
                    CyclesTopupMetrics::record_request_ok();
                    log!(
                        Topic::Cycles,
                        Ok,
                        "hub ICP self-refill completed operation_id={:?} cycles_sent={:?}",
                        response.operation_id,
                        response.cycles_sent
                    );
                }
                Ok(response) => {
                    CyclesTopupMetrics::record_request_err();
                    log!(
                        Topic::Cycles,
                        Warn,
                        "hub ICP self-refill advanced operation_id={:?} status={:?} error={:?}",
                        response.operation_id,
                        response.status,
                        response.error_code
                    );
                }
                Err(err) => {
                    CyclesTopupMetrics::record_request_err();
                    log!(Topic::Cycles, Error, "hub ICP self-refill failed: {err}");
                }
            }
        });

        true
    }

    fn check_auto_topup(cycles: &Cycles) {
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

        let topup_policy =
            canister_cfg
                .topup
                .as_ref()
                .map(|topup| policy::cycles::TopupPolicyInput {
                    threshold: topup.threshold.clone(),
                    amount: topup.amount.clone(),
                });

        let Some(plan) = policy::cycles::should_topup(cycles.to_u128(), topup_policy.as_ref())
        else {
            CyclesTopupMetrics::record_above_threshold();
            return;
        };

        let should_request = mark_in_flight(&TOPUP_IN_FLIGHT);

        if !should_request {
            CyclesTopupMetrics::record_request_in_flight();
            return;
        }

        CyclesTopupMetrics::record_request_scheduled();
        let requested_cycles = plan.amount;
        CycleTopupEventOps::record_scheduled(IcOps::now_secs(), requested_cycles.clone());
        IcOps::spawn(async move {
            let result = RequestOps::request_cycles(requested_cycles.to_u128()).await;

            clear_in_flight(&TOPUP_IN_FLIGHT);

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
                        IcOps::canister_cycle_balance()
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

        purged > 0 || purged_topups > 0
    }
}

fn mark_in_flight(flag: &'static LocalKey<RefCell<bool>>) -> bool {
    flag.with_borrow_mut(|in_flight| {
        if *in_flight {
            false
        } else {
            *in_flight = true;
            true
        }
    })
}

fn clear_in_flight(flag: &'static LocalKey<RefCell<bool>>) {
    flag.with_borrow_mut(|in_flight| {
        *in_flight = false;
    });
}
