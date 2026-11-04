//! Non-root bootstrap owns one native continuation across bounded authority retries.
//! Timers executes callbacks; lifecycle reconstruction starts a fresh bootstrap window.

use crate::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    lifecycle::{LifecyclePhase, retryable_nonroot_bootstrap_error},
    log,
    log::Topic,
    ops::runtime::bootstrap::{BootstrapPhaseLabel, BootstrapStatusOps},
    workflow::runtime::timer::{TimerError, require_active},
};
use ic_timers::{
    OnceDecision, OnceRegistration, OnceRunResult, TimerCompletion, TimerIdentity, TimerSchedule,
    TimerStart, register_once,
};
use std::{cell::RefCell, time::Duration};

const MAX_ATTEMPTS: u32 = 64;

thread_local! {
    static BOOTSTRAP: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
}

/// Start bootstrap without replacing an existing scheduled or running callback.
pub fn schedule(phase: LifecyclePhase) -> Result<(), TimerError> {
    require_active()?;
    BOOTSTRAP.with(|slot| {
        let mut slot = slot.try_borrow_mut().map_err(|_| TimerError::CustodyBusy)?;
        if let Some(registration) = slot.as_ref() {
            match registration.ensure_scheduled(TimerSchedule::After(Duration::ZERO)) {
                Ok(()) => return Ok(()),
                Err(ic_timers::TimerError::RegistrationExpired) => *slot = None,
                Err(error) => return Err(error.into()),
            }
        }
        let mut attempt: u32 = 0;
        *slot = Some(register_once(
            TimerIdentity::try_new("canic", "nonroot_bootstrap", "advance")?,
            TimerStart::RemoveWhenStopped {
                initial: TimerSchedule::After(Duration::ZERO),
            },
            move |_| {
                let current = attempt;
                attempt = attempt.saturating_add(1);
                crate::perf::with_async_context(run(phase, current))
            },
        )?);
        Ok(())
    })
}

async fn run(phase: LifecyclePhase, attempt: u32) -> OnceRunResult {
    if require_active().is_err() {
        return OnceRunResult::new(TimerCompletion::no_work(), OnceDecision::Stop);
    }
    let (metric, started, waiting) = match phase {
        LifecyclePhase::Init => (
            LifecycleMetricPhase::Init,
            BootstrapPhaseLabel::NONROOT_INIT,
            BootstrapPhaseLabel::NONROOT_INIT_WAITING_AUTHORITY,
        ),
        LifecyclePhase::PostUpgrade => (
            LifecycleMetricPhase::PostUpgrade,
            BootstrapPhaseLabel::NONROOT_UPGRADE,
            BootstrapPhaseLabel::NONROOT_UPGRADE_WAITING_AUTHORITY,
        ),
    };
    BootstrapStatusOps::set_phase(started);
    LifecycleMetricsApi::record_bootstrap(
        metric,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricOutcome::Started,
    );
    let result = match phase {
        LifecyclePhase::Init => super::nonroot::bootstrap_init_nonroot_canister().await,
        LifecyclePhase::PostUpgrade => {
            super::nonroot::bootstrap_post_upgrade_nonroot_canister().await
        }
    };
    match result {
        Ok(()) => {
            LifecycleMetricsApi::record_bootstrap(
                metric,
                LifecycleMetricRole::Nonroot,
                LifecycleMetricOutcome::Completed,
            );
            OnceRunResult::new(TimerCompletion::success(1), OnceDecision::Stop)
        }
        Err(error) => {
            let retryable = retryable_nonroot_bootstrap_error(&error);
            let decision = retry_decision(attempt, retryable);
            if decision == OnceDecision::Stop {
                LifecycleMetricsApi::record_bootstrap(
                    metric,
                    LifecycleMetricRole::Nonroot,
                    LifecycleMetricOutcome::Failed,
                );
                let failure = format!("non-root bootstrap failed ({phase}): {error}");
                if phase == LifecyclePhase::Init && retryable {
                    BootstrapStatusOps::mark_retryable_init_failure_exhausted(failure);
                } else {
                    BootstrapStatusOps::mark_failed(failure);
                }
                log!(
                    Topic::Init,
                    Error,
                    "non-root bootstrap failed ({phase}): {error}"
                );
            } else {
                BootstrapStatusOps::set_phase(waiting);
                log!(
                    Topic::Init,
                    Warn,
                    "non-root bootstrap waiting for managed authority ({phase}, attempt {}/{MAX_ATTEMPTS}): {error}",
                    attempt.saturating_add(1)
                );
            }
            let completion = if retryable {
                TimerCompletion::retryable_failure(0)
            } else {
                TimerCompletion::invariant_failure(0)
            };
            OnceRunResult::new(completion, decision)
        }
    }
}

fn retry_decision(attempt: u32, retryable: bool) -> OnceDecision {
    let next = attempt.saturating_add(1);
    if !retryable || next >= MAX_ATTEMPTS {
        return OnceDecision::Stop;
    }
    OnceDecision::RetryAfter(Duration::from_millis(250_u64 << next.min(4)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_stop_at_the_budget_and_hard_failures_never_rearm() {
        for attempt in 0..MAX_ATTEMPTS {
            assert_eq!(retry_decision(attempt, false), OnceDecision::Stop);
        }
        assert_eq!(
            retry_decision(0, true),
            OnceDecision::RetryAfter(Duration::from_millis(500))
        );
        assert_eq!(
            retry_decision(3, true),
            OnceDecision::RetryAfter(Duration::from_secs(4))
        );
        assert_eq!(
            retry_decision(MAX_ATTEMPTS - 2, true),
            OnceDecision::RetryAfter(Duration::from_secs(4))
        );
        assert_eq!(retry_decision(MAX_ATTEMPTS - 1, true), OnceDecision::Stop);
        assert_eq!(retry_decision(u32::MAX, true), OnceDecision::Stop);
    }
}
