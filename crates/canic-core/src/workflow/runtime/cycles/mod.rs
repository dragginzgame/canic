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
    domain::{
        icp_refill::{IcpRefillStatus, icp_refill_outcome_is_resumable},
        policy::pure as policy,
        runtime::TimerExecutionOutcome,
    },
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
    workflow::{
        ic::icp_refill::IcpRefillWorkflow,
        runtime::timer::{TimerDirective, TimerKey, TimerRunResult, TimerWorkflow},
    },
};
use std::time::Duration;
use thiserror::Error as ThisError;

const NANOS_PER_SECOND: u64 = 1_000_000_000;
const RETENTION_BATCH_SIZE: usize = 128;
const RETRY_INITIAL: Duration = Duration::from_mins(1);
const RETRY_MAX: Duration = Duration::from_mins(30);

#[derive(Clone)]
enum AutomaticTopupKind {
    Parent { amount: Cycles },
    RootIcp,
}

#[derive(Clone)]
struct AutomaticTopupConfig {
    threshold: u128,
    kind: AutomaticTopupKind,
}

struct CycleBalanceSample {
    timestamp_secs: u64,
    cycles: Cycles,
}

#[derive(Debug, ThisError)]
enum AutomaticTopupError {
    #[error(transparent)]
    Internal(#[from] InternalError),

    #[error(
        "hub ICP self-refill operation_id={operation_id:?} status={status:?} error={error_code:?}"
    )]
    RootOutcome {
        operation_id: [u8; 32],
        status: IcpRefillStatus,
        error_code: Option<crate::domain::icp_refill::IcpRefillErrorCode>,
        ledger_block_recorded: bool,
    },
}

impl AutomaticTopupError {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Internal(err) => is_retryable_funding_error(err),
            Self::RootOutcome {
                status,
                error_code,
                ledger_block_recorded,
                ..
            } => icp_refill_outcome_is_resumable(*status, *error_code, *ledger_block_recorded),
        }
    }
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

        if config.is_none() {
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

        let result = match &config.kind {
            AutomaticTopupKind::Parent { amount } => Self::request_parent_funding(amount).await,
            AutomaticTopupKind::RootIcp => Self::request_root_refill(&sample.cycles).await,
        };
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
                TimerRunResult::success(1, Self::directive(IcOps::now_nanos(), timing))
            }
            Err(failure) if failure.is_retryable() => {
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

    async fn request_parent_funding(amount: &Cycles) -> Result<(), AutomaticTopupError> {
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
                Err(err.into())
            }
        }
    }

    async fn request_root_refill(hub_cycles: &Cycles) -> Result<(), AutomaticTopupError> {
        CyclesTopupMetrics::record_request_scheduled();
        match IcpRefillWorkflow::execute_hub_self_refill(hub_cycles.clone()).await {
            Ok(response) if response.status == IcpRefillStatus::Completed => {
                CyclesTopupMetrics::record_request_ok();
                log!(
                    Topic::Cycles,
                    Ok,
                    "hub ICP self-refill completed operation_id={:?} cycles_sent={:?}",
                    response.operation_id,
                    response.cycles_sent
                );
                Ok(())
            }
            Ok(response) => {
                CyclesTopupMetrics::record_request_err();
                Err(AutomaticTopupError::RootOutcome {
                    operation_id: response.operation_id,
                    status: response.status,
                    error_code: response.error_code,
                    ledger_block_recorded: response.ledger_block_index.is_some(),
                })
            }
            Err(err) => {
                CyclesTopupMetrics::record_request_err();
                Err(err.into())
            }
        }
    }

    fn automatic_topup_config() -> Result<Option<AutomaticTopupConfig>, InternalError> {
        let canister = ConfigOps::current_canister()?;
        Ok(select_automatic_topup_config(
            EnvOps::is_root(),
            canister.topup,
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
) -> Option<AutomaticTopupConfig> {
    let topup = topup?;
    if is_root {
        let icp_refill = topup.icp_refill?;
        if !icp_refill.enabled {
            return None;
        }
        return Some(AutomaticTopupConfig {
            threshold: icp_refill.min_hub_cycles_before_refill.to_u128(),
            kind: AutomaticTopupKind::RootIcp,
        });
    }

    Some(AutomaticTopupConfig {
        threshold: topup.threshold.to_u128(),
        kind: AutomaticTopupKind::Parent {
            amount: topup.amount,
        },
    })
}

fn is_retryable_funding_error(err: &InternalError) -> bool {
    matches!(
        err.class(),
        InternalErrorClass::Infra | InternalErrorClass::Ops
    ) || err
        .public_error()
        .is_some_and(|err| matches!(err.code, ErrorCode::Conflict | ErrorCode::ResourceExhausted))
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
    use crate::{
        InternalErrorOrigin, config::schema::IcpRefillPolicy, dto::error::Error as PublicError,
    };

    #[test]
    fn automatic_topup_retry_backoff_is_bounded_and_deterministic() {
        assert_eq!(retry_delay(0), Duration::from_mins(1));
        assert_eq!(retry_delay(1), Duration::from_mins(2));
        assert_eq!(retry_delay(4), Duration::from_mins(16));
        assert_eq!(retry_delay(5), Duration::from_mins(30));
        assert_eq!(retry_delay(u64::MAX), Duration::from_mins(30));
    }

    #[test]
    fn only_transport_and_recoverable_public_funding_failures_retry() {
        assert!(is_retryable_funding_error(&InternalError::ops(
            InternalErrorOrigin::Ops,
            "transport"
        )));
        assert!(is_retryable_funding_error(&InternalError::public(
            PublicError::conflict("in flight")
        )));
        assert!(is_retryable_funding_error(&InternalError::public(
            PublicError::exhausted("cooldown")
        )));
        assert!(!is_retryable_funding_error(&InternalError::public(
            PublicError::forbidden("disabled")
        )));
        assert!(!is_retryable_funding_error(&InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "contradiction"
        )));

        assert!(
            AutomaticTopupError::RootOutcome {
                operation_id: [1; 32],
                status: IcpRefillStatus::Transferred,
                error_code: None,
                ledger_block_recorded: true,
            }
            .is_retryable()
        );
        assert!(
            !AutomaticTopupError::RootOutcome {
                operation_id: [2; 32],
                status: IcpRefillStatus::Failed,
                error_code: Some(crate::domain::icp_refill::IcpRefillErrorCode::NotifyMaxAttempts),
                ledger_block_recorded: true,
            }
            .is_retryable()
        );
    }

    #[test]
    fn configured_root_and_nonroot_select_their_single_funding_paths() {
        let topup = TopupPolicy {
            threshold: Cycles::new(10),
            amount: Cycles::new(5),
            icp_refill: Some(IcpRefillPolicy {
                enabled: true,
                min_hub_cycles_before_refill: Cycles::new(2),
                max_refill_e8s_per_call: 100,
                min_xdr_permyriad_per_icp: None,
                ledger_canister_id: None,
                cmc_canister_id: None,
                allow_ic_system_canister_overrides: false,
            }),
        };

        let root = select_automatic_topup_config(true, Some(topup.clone()))
            .expect("configured root policy");
        assert_eq!(root.threshold, 2);
        assert!(matches!(root.kind, AutomaticTopupKind::RootIcp));

        let nonroot =
            select_automatic_topup_config(false, Some(topup)).expect("configured parent policy");
        assert_eq!(nonroot.threshold, 10);
        assert!(matches!(
            nonroot.kind,
            AutomaticTopupKind::Parent { amount } if amount == Cycles::new(5)
        ));
        assert!(select_automatic_topup_config(true, Some(TopupPolicy::default())).is_none());
        assert!(select_automatic_topup_config(false, None).is_none());
    }
}
