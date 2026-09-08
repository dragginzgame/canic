//! Module: workflow::metrics::publication::timer
//!
//! Responsibility: own one optional public sampling task in the native timer runtime.
//! Does not own: producer state, history storage, or funding decisions.
//! Boundary: selected families enable one five-minute task; missed slots stay gaps.

use crate::{
    InternalError,
    model::public_metrics::PUBLIC_METRICS_CADENCE_NS,
    ops::{ic::IcOps, runtime::public_metrics::PublicMetricsOps},
    workflow::{
        metrics::publication::PublicMetricsWorkflow,
        runtime::timer::{TimerError, require_active, retain_owned_once, with_owned_once},
    },
};
use ic_timers::{
    DeclarationLifetime, OnceRegistration, TimerCompletion, TimerDirective, TimerIdentity,
    TimerRunResult, TimerSchedule, register_once,
};
use std::cell::RefCell;

thread_local! {
    static SAMPLING_TIMER: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
}

/// The sole optional sampling claim, independent of the financial cycle owner.
pub struct PublicSamplingTimer;

impl PublicSamplingTimer {
    pub(crate) fn timer_identity() -> Result<TimerIdentity, TimerError> {
        TimerIdentity::try_new("canic", "public_metrics", "sample").map_err(Into::into)
    }

    pub(crate) fn claimed_timer_identity() -> Result<Option<TimerIdentity>, TimerError> {
        with_owned_once(&SAMPLING_TIMER, |registration| {
            registration.identity().clone()
        })
    }

    pub(crate) fn cancel_timer() -> Result<(), TimerError> {
        if let Some(result) = with_owned_once(&SAMPLING_TIMER, OnceRegistration::cancel)? {
            result?;
        }
        Ok(())
    }

    /// Start or reconcile one retained claim only when publication is selected.
    pub fn start() -> Result<(), InternalError> {
        if PublicMetricsOps::enabled().is_empty() {
            Self::cancel_timer()?;
            return Ok(());
        }
        require_active()?;
        if with_owned_once(&SAMPLING_TIMER, |_| ())?.is_none() {
            let registration = register_once(
                Self::timer_identity()?,
                DeclarationLifetime::Retained,
                |_context| async { Self::run() },
            )
            .map_err(TimerError::from)?;
            retain_owned_once(&SAMPLING_TIMER, registration)?;
        }
        let deadline = next_deadline(IcOps::now_nanos());
        with_owned_once(&SAMPLING_TIMER, |registration| {
            registration.reconcile_schedule(deadline.map(TimerSchedule::At))
        })?
        .ok_or(TimerError::MissingClaim)?
        .map_err(TimerError::from)?;
        Ok(())
    }

    fn run() -> TimerRunResult {
        if require_active().is_err() || PublicMetricsOps::enabled().is_empty() {
            return TimerRunResult::new(TimerCompletion::no_work(), TimerDirective::Stop);
        }
        let result = PublicMetricsWorkflow::sample();
        let completion = if result.is_ok() {
            TimerCompletion::success(1)
        } else {
            TimerCompletion::invariant_failure(0)
        };
        let directive = next_deadline(IcOps::now_nanos())
            .map_or(TimerDirective::Stop, TimerDirective::ScheduleAt);
        TimerRunResult::new(completion, directive)
    }
}

fn next_deadline(now_ns: u64) -> Option<u64> {
    (now_ns / PUBLIC_METRICS_CADENCE_NS)
        .checked_add(1)?
        .checked_mul(PUBLIC_METRICS_CADENCE_NS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delayed_sampling_skips_missed_slots_and_overflow_stops() {
        assert_eq!(next_deadline(0), Some(PUBLIC_METRICS_CADENCE_NS));
        assert_eq!(
            next_deadline(20 * PUBLIC_METRICS_CADENCE_NS + 9),
            Some(21 * PUBLIC_METRICS_CADENCE_NS)
        );
        assert_eq!(next_deadline(u64::MAX), None);
    }
}
