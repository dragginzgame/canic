//! Module: workflow::runtime::auth::renewal
//!
//! Responsibility: schedule root-managed delegated-proof renewal sweeps.
//! Does not own: renewal policy, proof preparation internals, or issuer installs.

use crate::{
    InternalError, InternalErrorClass, InternalErrorOrigin,
    config::schema::DelegatedTokenConfig,
    domain::runtime::{FailureSeverity, TimerExecutionOutcome},
    dto::error::ErrorCode,
    log,
    log::Topic,
    ops::{
        auth::{AuthOps, PrepareChainKeyRootDelegationBatchInput},
        config::ConfigOps,
        ic::IcOps,
        runtime::{
            env::EnvOps,
            metrics::delegated_auth::DelegatedAuthMetrics,
            recent_failure::{RecentFailureInput, RecentFailureOps},
        },
    },
    workflow::runtime::{
        auth::{provisioning, root_delegation_batch},
        timer::{TimerDirective, TimerKey, TimerRunResult, TimerWorkflow},
    },
};
use std::time::Duration;

const DEFAULT_DELEGATED_TOKEN_MAX_TTL_SECS: u64 = 24 * 60 * 60;
const NANOS_PER_SECOND: u64 = 1_000_000_000;
const PROOF_VALIDITY_RETRY_FLOOR: Duration = Duration::from_secs(1);
const RETRY_INITIAL: Duration = Duration::from_mins(1);
const RETRY_MAX: Duration = Duration::from_mins(30);

struct RenewalSweepFailure {
    cause: InternalError,
    work_count: u64,
}

pub(super) struct RootIssuerRenewalWorkflow;

impl RootIssuerRenewalWorkflow {
    pub(super) fn reconcile() -> Result<(), InternalError> {
        if !EnvOps::is_root() {
            return Ok(());
        }
        if !AuthOps::has_enabled_root_issuer_renewal_templates() {
            Self::reconcile_deadline(None);
            return Ok(());
        }
        let config = ConfigOps::delegated_tokens_config()?;
        if !config.enabled {
            log!(
                Topic::Auth,
                Warn,
                "root delegated-proof renewal timer skipped: delegated-token auth is disabled"
            );
            Self::reconcile_deadline(None);
            return Ok(());
        }
        require_chain_key_root_proof_mode(&config)?;

        let timing = AuthOps::root_issuer_renewal_timing(IcOps::now_nanos())?;
        Self::reconcile_deadline(timing.next_deadline_ns);

        Ok(())
    }

    async fn run_scheduled() -> TimerRunResult {
        let now_ns = IcOps::now_nanos();
        match Self::sweep().await {
            Ok(work_count) => Self::completed_result(now_ns, work_count),
            Err(failure) => Self::failed_result(now_ns, failure),
        }
    }

    async fn sweep() -> Result<u64, RenewalSweepFailure> {
        if !AuthOps::has_enabled_root_issuer_renewal_templates() {
            return Ok(0);
        }

        DelegatedAuthMetrics::record_renewal_sweep_started();
        let result = Self::sweep_configured().await;
        match &result {
            Ok(_) => DelegatedAuthMetrics::record_renewal_sweep_completed(),
            Err(_) => DelegatedAuthMetrics::record_renewal_sweep_failed(),
        }
        result
    }

    async fn sweep_configured() -> Result<u64, RenewalSweepFailure> {
        let config = ConfigOps::delegated_tokens_config().map_err(RenewalSweepFailure::new)?;
        if !config.enabled {
            return Ok(0);
        }
        require_chain_key_root_proof_mode(&config).map_err(RenewalSweepFailure::new)?;
        let build_network = config.build_network;
        let min_accepted_proof_epoch =
            chain_key_min_accepted_proof_epoch(&config).map_err(RenewalSweepFailure::new)?;
        let max_cert_ttl_ns = delegated_token_max_ttl_ns().map_err(RenewalSweepFailure::new)?;
        let now_ns = IcOps::now_nanos();
        let prepared = root_delegation_batch::prepare_due_chain_key_root_delegation_batch(
            PrepareChainKeyRootDelegationBatchInput {
                build_network,
                max_cert_ttl_ns,
                min_accepted_proof_epoch,
                required_issuer_pid: None,
                now_ns,
            },
        )
        .map_err(RenewalSweepFailure::new)?;
        let mut work_count = if prepared.reused_in_flight {
            0
        } else {
            u64::try_from(prepared.prepared_issuers).map_err(|_| {
                RenewalSweepFailure::new(InternalError::invariant(
                    InternalErrorOrigin::Workflow,
                    "delegated-proof renewal work count exceeded u64",
                ))
            })?
        };
        let signed = AuthOps::sign_next_chain_key_root_delegation_batch(build_network, now_ns)
            .await
            .map_err(|cause| RenewalSweepFailure { cause, work_count })?;
        if signed.signed {
            work_count = checked_work_count(work_count, 1)?;
        }
        if let Some(request) = AuthOps::start_next_chain_key_root_delegation_batch_install(now_ns)
            .map_err(|cause| RenewalSweepFailure { cause, work_count })?
        {
            let outcome =
                provisioning::install_chain_key_delegation_proof_batch(request, now_ns).await;
            work_count = checked_work_count(work_count, outcome.installed_count)?;
            if let Some(cause) = outcome.failure {
                return Err(RenewalSweepFailure { cause, work_count });
            }
        }

        if let Some(batch_id) = prepared.batch_id {
            log!(
                Topic::Auth,
                Info,
                "root chain-key delegated-proof renewal prepared batch_id={:?} issuers={} skipped={}",
                batch_id,
                prepared.prepared_issuers,
                prepared.skipped_templates
            );
        }

        Ok(work_count)
    }

    fn completed_result(now_ns: u64, work_count: u64) -> TimerRunResult {
        let timing = match AuthOps::root_issuer_renewal_timing(now_ns) {
            Ok(timing) => timing,
            Err(err) => {
                Self::record_timer_failure(&err);
                return TimerRunResult {
                    outcome: TimerExecutionOutcome::InvariantFailure,
                    work_count,
                    directive: TimerDirective::Stop,
                };
            }
        };
        let directive = match timing.next_deadline_ns {
            None => TimerDirective::Stop,
            Some(deadline_ns) if deadline_ns > now_ns => TimerDirective::ScheduleAt(deadline_ns),
            Some(_) if work_count > 0 => TimerDirective::ContinueImmediately,
            Some(_) => {
                let err = InternalError::invariant(
                    InternalErrorOrigin::Workflow,
                    "delegated-proof renewal remained due without making progress",
                );
                Self::record_timer_failure(&err);
                return TimerRunResult::invariant_failure();
            }
        };

        if work_count == 0 {
            TimerRunResult::no_work(directive)
        } else {
            TimerRunResult::success(work_count, directive)
        }
    }

    fn failed_result(now_ns: u64, failure: RenewalSweepFailure) -> TimerRunResult {
        Self::record_timer_failure(&failure.cause);
        if !is_retryable_renewal_error(&failure.cause) {
            return TimerRunResult {
                outcome: TimerExecutionOutcome::InvariantFailure,
                work_count: failure.work_count,
                directive: TimerDirective::Stop,
            };
        }

        let active_proof_expires_at_ns = match AuthOps::root_issuer_renewal_timing(now_ns) {
            Ok(timing) => timing.earliest_active_proof_expires_at_ns,
            Err(err) => {
                Self::record_timer_failure(&err);
                return TimerRunResult {
                    outcome: TimerExecutionOutcome::InvariantFailure,
                    work_count: failure.work_count,
                    directive: TimerDirective::Stop,
                };
            }
        };
        let streak = TimerWorkflow::consecutive_expected_failures(TimerKey::AuthRenewal);
        let mut delay = retry_delay(streak, now_ns, active_proof_expires_at_ns);
        let Some(retry_after_ns) = retry_deadline_ns(now_ns, delay) else {
            let err = InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "delegated-proof renewal retry deadline overflowed",
            );
            Self::record_timer_failure(&err);
            return TimerRunResult {
                outcome: TimerExecutionOutcome::InvariantFailure,
                work_count: failure.work_count,
                directive: TimerDirective::Stop,
            };
        };
        let deferred = match AuthOps::defer_retryable_chain_key_root_delegation_batch(
            now_ns,
            retry_after_ns,
        ) {
            Ok(deferred) => deferred,
            Err(err) => {
                Self::record_timer_failure(&err);
                return TimerRunResult {
                    outcome: TimerExecutionOutcome::InvariantFailure,
                    work_count: failure.work_count,
                    directive: TimerDirective::Stop,
                };
            }
        };
        if deferred {
            let exact_timing = match AuthOps::root_issuer_renewal_timing(now_ns) {
                Ok(timing) => timing,
                Err(err) => {
                    Self::record_timer_failure(&err);
                    return TimerRunResult {
                        outcome: TimerExecutionOutcome::InvariantFailure,
                        work_count: failure.work_count,
                        directive: TimerDirective::Stop,
                    };
                }
            };
            let Some(exact_deadline_ns) = exact_timing.next_deadline_ns else {
                let err = InternalError::invariant(
                    InternalErrorOrigin::Workflow,
                    "retryable delegated-proof batch lost its durable deadline",
                );
                Self::record_timer_failure(&err);
                return TimerRunResult {
                    outcome: TimerExecutionOutcome::InvariantFailure,
                    work_count: failure.work_count,
                    directive: TimerDirective::Stop,
                };
            };
            let Some(exact_delay_ns) = exact_deadline_ns.checked_sub(now_ns) else {
                let err = InternalError::invariant(
                    InternalErrorOrigin::Workflow,
                    "durable delegated-proof retry deadline preceded the timing observation",
                );
                Self::record_timer_failure(&err);
                return TimerRunResult {
                    outcome: TimerExecutionOutcome::InvariantFailure,
                    work_count: failure.work_count,
                    directive: TimerDirective::Stop,
                };
            };
            delay = Duration::from_nanos(exact_delay_ns);
        }
        TimerRunResult {
            outcome: TimerExecutionOutcome::RetryableFailure,
            work_count: failure.work_count,
            directive: TimerDirective::RetryAfter(delay),
        }
    }

    fn reconcile_deadline(deadline_ns: Option<u64>) {
        TimerWorkflow::reconcile_at(TimerKey::AuthRenewal, deadline_ns, || async {
            Self::run_scheduled().await
        });
    }

    fn record_timer_failure(err: &InternalError) {
        let (class, origin) = err.log_fields();
        RecentFailureOps::record(RecentFailureInput {
            occurred_at_ns: IcOps::now_nanos(),
            subsystem: "auth_renewal".to_string(),
            code: renewal_failure_code(class, origin),
            severity: FailureSeverity::Error,
            summary: format!("class={class} origin={origin}: {err}"),
            correlation_id: None,
        });
        log!(
            Topic::Auth,
            Warn,
            "root delegated-proof renewal sweep failed class={class} origin={origin}: {err}"
        );
    }
}

impl RenewalSweepFailure {
    const fn new(cause: InternalError) -> Self {
        Self {
            cause,
            work_count: 0,
        }
    }
}

fn checked_work_count(work_count: u64, additional_work: u64) -> Result<u64, RenewalSweepFailure> {
    work_count
        .checked_add(additional_work)
        .ok_or_else(|| RenewalSweepFailure {
            cause: InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "delegated-proof renewal work count overflowed",
            ),
            work_count,
        })
}

fn is_retryable_renewal_error(err: &InternalError) -> bool {
    matches!(
        err.class(),
        InternalErrorClass::Infra | InternalErrorClass::Ops
    ) || err.public_error().is_some_and(|err| {
        matches!(
            err.code,
            ErrorCode::AuthProofPending | ErrorCode::Conflict | ErrorCode::Unavailable
        )
    })
}

fn retry_delay(streak: u64, now_ns: u64, active_proof_expires_at_ns: Option<u64>) -> Duration {
    let exponent = u32::try_from(streak.min(5)).expect("bounded retry exponent must fit u32");
    let multiplier = 1u32 << exponent;
    let backoff = RETRY_INITIAL
        .checked_mul(multiplier)
        .expect("bounded auth retry duration must fit")
        .min(RETRY_MAX);
    let Some(remaining_ns) =
        active_proof_expires_at_ns.and_then(|expires_at_ns| expires_at_ns.checked_sub(now_ns))
    else {
        return backoff;
    };
    let proof_delay = if remaining_ns > 2 * NANOS_PER_SECOND {
        Duration::from_nanos(
            (remaining_ns / 2).max(
                u64::try_from(PROOF_VALIDITY_RETRY_FLOOR.as_nanos())
                    .expect("proof retry floor must fit u64 nanoseconds"),
            ),
        )
    } else {
        Duration::from_nanos(remaining_ns.max(1))
    };
    backoff.min(proof_delay)
}

fn retry_deadline_ns(now_ns: u64, delay: Duration) -> Option<u64> {
    let delay_ns = u64::try_from(delay.as_nanos()).ok()?;
    now_ns.checked_add(delay_ns)
}

fn renewal_failure_code(class: InternalErrorClass, origin: InternalErrorOrigin) -> String {
    format!(
        "renewal_sweep_failed/{}/{}",
        internal_error_class_code(class),
        internal_error_origin_code(origin)
    )
}

const fn internal_error_class_code(class: InternalErrorClass) -> &'static str {
    match class {
        InternalErrorClass::Access => "access",
        InternalErrorClass::Domain => "domain",
        InternalErrorClass::Infra => "infra",
        InternalErrorClass::Ops => "ops",
        InternalErrorClass::Workflow => "workflow",
        InternalErrorClass::Invariant => "invariant",
    }
}

const fn internal_error_origin_code(origin: InternalErrorOrigin) -> &'static str {
    match origin {
        InternalErrorOrigin::Access => "access",
        InternalErrorOrigin::Config => "config",
        InternalErrorOrigin::Domain => "domain",
        InternalErrorOrigin::Infra => "infra",
        InternalErrorOrigin::Ops => "ops",
        InternalErrorOrigin::Storage => "storage",
        InternalErrorOrigin::Workflow => "workflow",
    }
}

fn require_chain_key_root_proof_mode(config: &DelegatedTokenConfig) -> Result<(), InternalError> {
    if config.root_proof_mode.trim() == "chain_key_batch" {
        return Ok(());
    }
    Err(InternalError::invariant(
        InternalErrorOrigin::Workflow,
        "delegated-auth renewal requires root_proof_mode=\"chain_key_batch\"",
    ))
}

fn chain_key_min_accepted_proof_epoch(config: &DelegatedTokenConfig) -> Result<u64, InternalError> {
    config
        .chain_key_root_proof
        .min_accepted_proof_epoch
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "auth.delegated_tokens.chain_key_root_proof.min_accepted_proof_epoch is required for chain-key renewal",
            )
        })
}

fn delegated_token_max_ttl_ns() -> Result<u64, InternalError> {
    let cfg = ConfigOps::delegated_tokens_config()?;
    let max_ttl_secs = cfg
        .max_ttl_secs
        .unwrap_or(DEFAULT_DELEGATED_TOKEN_MAX_TTL_SECS);
    max_ttl_secs.checked_mul(1_000_000_000).ok_or_else(|| {
        InternalError::invalid_input("auth.delegated_tokens.max_ttl_secs overflows nanoseconds")
    })
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::seams;

    #[test]
    fn timer_failure_preserves_typed_classification_in_recent_diagnostics() {
        let _guard = seams::lock();
        RecentFailureOps::reset();
        let err = InternalError::invariant(InternalErrorOrigin::Workflow, "renewal invariant");

        RootIssuerRenewalWorkflow::record_timer_failure(&err);

        let failures = RecentFailureOps::snapshot();
        assert_eq!(err.class(), InternalErrorClass::Invariant);
        assert_eq!(err.origin(), InternalErrorOrigin::Workflow);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].subsystem, "auth_renewal");
        assert_eq!(failures[0].code, "renewal_sweep_failed/invariant/workflow");
        assert_eq!(failures[0].severity, FailureSeverity::Error);
        RecentFailureOps::reset();
    }

    #[test]
    fn retry_backoff_is_bounded_and_proof_aware() {
        assert_eq!(retry_delay(0, 100, None), Duration::from_mins(1));
        assert_eq!(retry_delay(8, 100, None), RETRY_MAX);
        assert_eq!(
            retry_delay(0, 100, Some(20 * NANOS_PER_SECOND + 100)),
            Duration::from_secs(10)
        );
        assert_eq!(
            retry_delay(0, 100, Some(NANOS_PER_SECOND + 100)),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn retry_deadline_overflow_is_explicit() {
        assert_eq!(retry_deadline_ns(u64::MAX, Duration::from_nanos(1)), None);
    }

    #[test]
    fn work_count_overflow_is_an_invariant_failure() {
        let failure = checked_work_count(u64::MAX, 1)
            .expect_err("unrepresentable work count must fail closed");

        assert_eq!(failure.cause.class(), InternalErrorClass::Invariant);
        assert_eq!(failure.work_count, u64::MAX);
    }
}
