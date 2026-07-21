//! Module: workflow::runtime::cycles
//!
//! Responsibility: record cycle observations and run configured automatic funding.
//! Does not own: funding policy, stable telemetry schemas, or timer arbitration.
//! Boundary: lifecycle and funding events record history; one timer owns top-up safety.

pub mod query;

use crate::{
    InternalError, InternalErrorClass,
    cdk::types::Cycles,
    config::schema::TopupPolicy,
    domain::{policy::pure as policy, runtime::TimerExecutionOutcome},
    dto::error::ErrorCode,
    log,
    log::Topic,
    ops::{
        config::ConfigOps,
        ic::IcOps,
        rpc::request::RequestOps,
        runtime::{env::EnvOps, metrics::cycles_topup::CyclesTopupMetrics},
        storage::cycles::{CycleTopupEventOps, CycleTrackerOps},
    },
    workflow::runtime::timer::{TimerDirective, TimerKey, TimerRunResult, TimerWorkflow},
};
use std::time::Duration;

const NANOS_PER_SECOND: u64 = 1_000_000_000;
const RETENTION_BATCH_SIZE: usize = 128;
const RETRY_INITIAL: Duration = Duration::from_mins(1);
const RETRY_MAX: Duration = Duration::from_mins(30);

struct AutomaticTopupConfig {
    threshold: u128,
    amount: Cycles,
    minimum_funding_spacing_secs: u64,
}

struct CycleBalanceSample {
    timestamp_secs: u64,
    cycles: Cycles,
}

/// Runtime owner for cycle observations and configured automatic funding.
pub struct CycleWorkflow;

impl CycleWorkflow {
    /// Record the lifecycle balance and reconcile the sole top-up safety deadline.
    pub fn start() -> Result<(), InternalError> {
        let config = Self::automatic_topup_config()?;
        let previous = Self::latest_observation();
        let sample = Self::read_sample();
        Self::record_observation(&sample);

        if config.is_none() && !EnvOps::is_root() {
            CyclesTopupMetrics::record_policy_missing();
        }
        let deadline = config.as_ref().map(|config| {
            Self::deadline_ns(
                IcOps::now_nanos(),
                policy::cycles::cycle_topup_timing(
                    sample.timestamp_secs,
                    sample.cycles.to_u128(),
                    config.threshold,
                    previous,
                ),
            )
        });
        TimerWorkflow::reconcile_at(TimerKey::CycleTopup, deadline, || async {
            Self::run_topup().await
        });
        Ok(())
    }

    async fn run_topup() -> TimerRunResult {
        let config = match Self::automatic_topup_config() {
            Ok(Some(config)) => config,
            Ok(None) => return TimerRunResult::no_work(TimerDirective::Stop),
            Err(err) => {
                CyclesTopupMetrics::record_config_error();
                log!(Topic::Cycles, Error, "automatic top-up stopped: {err}");
                return TimerRunResult::invariant_failure();
            }
        };
        let previous = Self::latest_observation();
        let sample = Self::read_sample();
        Self::record_observation(&sample);
        let timing = policy::cycles::cycle_topup_timing(
            sample.timestamp_secs,
            sample.cycles.to_u128(),
            config.threshold,
            previous,
        );

        if !matches!(timing, policy::cycles::CycleTopupTiming::Due) {
            CyclesTopupMetrics::record_above_threshold();
            return TimerRunResult::no_work(Self::directive(IcOps::now_nanos(), timing));
        }

        let result = Self::request_parent_funding(&config.amount).await;
        let after = Self::read_sample();
        Self::record_observation(&after);

        match result {
            Ok(()) => {
                let timing = policy::cycles::cycle_topup_timing(
                    after.timestamp_secs,
                    after.cycles.to_u128(),
                    config.threshold,
                    Some(policy::cycles::CycleBalanceObservation {
                        timestamp_secs: sample.timestamp_secs,
                        balance: sample.cycles.to_u128(),
                    }),
                );
                let directive = if matches!(timing, policy::cycles::CycleTopupTiming::Due) {
                    TimerDirective::ScheduleAt(
                        IcOps::now_nanos().saturating_add(
                            config
                                .minimum_funding_spacing_secs
                                .saturating_mul(NANOS_PER_SECOND),
                        ),
                    )
                } else {
                    Self::directive(IcOps::now_nanos(), timing)
                };
                TimerRunResult::success(1, directive)
            }
            Err(failure) if is_retryable_funding_error(&failure) => {
                log!(
                    Topic::Cycles,
                    Warn,
                    "automatic top-up will retry: {}",
                    failure
                );
                let streak = TimerWorkflow::consecutive_expected_failures(TimerKey::CycleTopup);
                TimerRunResult {
                    outcome: TimerExecutionOutcome::RetryableFailure,
                    work_count: 0,
                    directive: TimerDirective::RetryAfter(retry_delay(streak)),
                }
            }
            Err(failure) => {
                log!(
                    Topic::Cycles,
                    Error,
                    "automatic top-up stopped: {}",
                    failure
                );
                TimerRunResult::invariant_failure()
            }
        }
    }

    async fn request_parent_funding(amount: &Cycles) -> Result<(), InternalError> {
        CyclesTopupMetrics::record_request_scheduled();
        CycleTopupEventOps::record_scheduled(IcOps::now_secs(), amount.clone());
        match RequestOps::request_cycles(amount.to_u128()).await {
            Ok(response) => {
                let transferred = Cycles::from(response.cycles_transferred);
                CyclesTopupMetrics::record_request_ok();
                CycleTopupEventOps::record_ok(
                    IcOps::now_secs(),
                    amount.clone(),
                    transferred.clone(),
                );
                log!(
                    Topic::Cycles,
                    Ok,
                    "requested {amount}, topped up by {transferred}, now {}",
                    IcOps::canister_cycle_balance()
                );
                Ok(())
            }
            Err(err) => {
                CyclesTopupMetrics::record_request_err();
                CycleTopupEventOps::record_err(IcOps::now_secs(), amount.clone(), err.to_string());
                Err(err)
            }
        }
    }

    fn automatic_topup_config() -> Result<Option<AutomaticTopupConfig>, InternalError> {
        let canister = ConfigOps::current_canister()?;
        Ok(select_automatic_topup_config(
            EnvOps::is_root(),
            canister.topup,
            canister.cycles_funding.cooldown_secs,
        ))
    }

    fn read_sample() -> CycleBalanceSample {
        CycleBalanceSample {
            timestamp_secs: IcOps::now_secs(),
            cycles: IcOps::canister_cycle_balance(),
        }
    }

    fn latest_observation() -> Option<policy::cycles::CycleBalanceObservation> {
        CycleTrackerOps::latest().map(|(timestamp_secs, cycles)| {
            policy::cycles::CycleBalanceObservation {
                timestamp_secs,
                balance: cycles.to_u128(),
            }
        })
    }

    fn record_observation(sample: &CycleBalanceSample) {
        CycleTrackerOps::record(sample.timestamp_secs, sample.cycles.clone());
        Self::purge_history(sample.timestamp_secs);
    }

    fn purge_history(now_secs: u64) {
        let cutoff = policy::cycles::retention_cutoff(now_secs);
        let purged_tracker = CycleTrackerOps::purge_before(cutoff, RETENTION_BATCH_SIZE);
        let purged_topups = CycleTopupEventOps::purge_before(cutoff, RETENTION_BATCH_SIZE);
        if purged_tracker > 0 || purged_topups > 0 {
            log!(
                Topic::Cycles,
                Info,
                "cycle history: purged {purged_tracker} balance entries and {purged_topups} top-up events"
            );
        }
    }

    const fn deadline_ns(now_ns: u64, timing: policy::cycles::CycleTopupTiming) -> u64 {
        match timing {
            policy::cycles::CycleTopupTiming::Due => now_ns,
            policy::cycles::CycleTopupTiming::CheckAfter { delay_secs } => {
                now_ns.saturating_add(delay_secs.saturating_mul(NANOS_PER_SECOND))
            }
        }
    }

    const fn directive(now_ns: u64, timing: policy::cycles::CycleTopupTiming) -> TimerDirective {
        match timing {
            policy::cycles::CycleTopupTiming::Due => {
                TimerDirective::ScheduleAt(now_ns.saturating_add(
                    policy::cycles::CYCLE_TOPUP_MIN_CHECK_SECS.saturating_mul(NANOS_PER_SECOND),
                ))
            }
            policy::cycles::CycleTopupTiming::CheckAfter { .. } => {
                TimerDirective::ScheduleAt(Self::deadline_ns(now_ns, timing))
            }
        }
    }
}

fn select_automatic_topup_config(
    is_root: bool,
    topup: Option<TopupPolicy>,
    funding_cooldown_secs: u64,
) -> Option<AutomaticTopupConfig> {
    if is_root {
        return None;
    }

    let topup = topup?;
    Some(AutomaticTopupConfig {
        threshold: topup.threshold.to_u128(),
        amount: topup.amount,
        minimum_funding_spacing_secs: funding_cooldown_secs
            .max(policy::cycles::CYCLE_TOPUP_MIN_CHECK_SECS),
    })
}

fn is_retryable_funding_error(err: &InternalError) -> bool {
    matches!(
        err.class(),
        InternalErrorClass::Infra | InternalErrorClass::Ops
    ) || err
        .public_error()
        .is_some_and(|err| err.code == ErrorCode::Conflict)
}

fn retry_delay(streak: u64) -> Duration {
    let exponent = u32::try_from(streak.min(5)).unwrap_or(5);
    let multiplier = 1u32 << exponent;
    RETRY_INITIAL
        .checked_mul(multiplier)
        .unwrap_or(RETRY_MAX)
        .min(RETRY_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InternalErrorOrigin, dto::error::Error as PublicError};

    #[test]
    fn automatic_topup_retry_backoff_is_bounded_and_deterministic() {
        assert_eq!(retry_delay(0), Duration::from_mins(1));
        assert_eq!(retry_delay(1), Duration::from_mins(2));
        assert_eq!(retry_delay(4), Duration::from_mins(16));
        assert_eq!(retry_delay(5), Duration::from_mins(30));
        assert_eq!(retry_delay(u64::MAX), Duration::from_mins(30));
    }

    #[test]
    fn only_transport_and_in_flight_funding_failures_retry() {
        assert!(is_retryable_funding_error(&InternalError::ops(
            InternalErrorOrigin::Ops,
            "transport"
        )));
        assert!(is_retryable_funding_error(&InternalError::public(
            PublicError::conflict("in flight")
        )));
        assert!(!is_retryable_funding_error(&InternalError::public(
            PublicError::exhausted("cooldown")
        )));
        assert!(!is_retryable_funding_error(&InternalError::public(
            PublicError::forbidden("disabled")
        )));
        assert!(!is_retryable_funding_error(&InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "contradiction"
        )));
    }

    #[test]
    fn automatic_topup_is_parent_funded_for_nonroot_only() {
        let topup = TopupPolicy {
            threshold: Cycles::new(10),
            amount: Cycles::new(5),
            icp_refill: None,
        };

        assert!(select_automatic_topup_config(true, Some(topup.clone()), 60).is_none());

        let nonroot = select_automatic_topup_config(false, Some(topup), 300)
            .expect("configured parent policy");
        assert_eq!(nonroot.threshold, 10);
        assert_eq!(nonroot.amount, Cycles::new(5));
        assert_eq!(nonroot.minimum_funding_spacing_secs, 300);
        assert!(select_automatic_topup_config(false, None, 60).is_none());
    }

    #[test]
    fn automatic_topup_spacing_never_undercuts_the_observation_floor() {
        let nonroot = select_automatic_topup_config(false, Some(TopupPolicy::default()), 0)
            .expect("configured parent policy");

        assert_eq!(
            nonroot.minimum_funding_spacing_secs,
            policy::cycles::CYCLE_TOPUP_MIN_CHECK_SECS
        );
    }
}
