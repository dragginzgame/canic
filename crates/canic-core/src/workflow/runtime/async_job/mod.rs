//! Module: workflow::runtime::async_job
//!
//! Responsibility: apply the shared serial-attempt lease protocol to domain async jobs.
//! Does not own: timer registration, provider state, domain demand, or external effects.
//! Boundary: exact owners claim and finish attempts around their own asynchronous work.

use crate::ops::{
    ic::IcOps,
    storage::async_job_recovery::{
        AsyncJobAttempt, AsyncJobClaim, AsyncJobCompletion, AsyncJobOwner, AsyncJobRecoveryOps,
    },
};
use ic_timers::{TimerCompletion, TimerCompletionOutcome, TimerDirective, TimerRunResult};

const ASYNC_JOB_LEASE_NS: u64 = 5 * 60 * 1_000_000_000;

/// Shared attempt-fence workflow used by the closed set of domain job owners.
pub struct AsyncJobWorkflow;

impl AsyncJobWorkflow {
    /// Return whether one owner currently retains an active business-attempt lease.
    #[must_use]
    pub fn has_active_attempt(owner: AsyncJobOwner) -> bool {
        AsyncJobRecoveryOps::active_lease_deadline(owner).is_some()
    }

    /// Return whether one owner has an expired business-attempt lease to recover.
    #[must_use]
    pub fn has_expired_attempt(owner: AsyncJobOwner, now_ns: u64) -> bool {
        AsyncJobRecoveryOps::expired_deadline(owner, now_ns).is_some()
    }

    /// Claim one ordinary callback attempt or return its exact provider disposition.
    pub fn claim(owner: AsyncJobOwner) -> Result<AsyncJobAttempt, TimerRunResult> {
        let now_ns = IcOps::now_nanos();
        let Some(lease_expires_at_ns) = now_ns.checked_add(ASYNC_JOB_LEASE_NS) else {
            return Err(invariant_failure());
        };
        match AsyncJobRecoveryOps::claim(owner, now_ns, lease_expires_at_ns) {
            Ok(AsyncJobClaim::Acquired(attempt)) => Ok(attempt),
            Ok(AsyncJobClaim::Busy { retry_at_ns }) => Err(TimerRunResult::new(
                TimerCompletion::retryable_failure(0),
                TimerDirective::ScheduleAt(retry_at_ns),
            )),
            Err(_) => Err(invariant_failure()),
        }
    }

    /// Claim only an expired attempt for owner-specific watchdog recovery.
    pub fn claim_expired(owner: AsyncJobOwner, now_ns: u64) -> Option<AsyncJobAttempt> {
        if !Self::has_expired_attempt(owner, now_ns) {
            return None;
        }
        let lease_expires_at_ns = now_ns.checked_add(ASYNC_JOB_LEASE_NS)?;
        match AsyncJobRecoveryOps::claim(owner, now_ns, lease_expires_at_ns) {
            Ok(AsyncJobClaim::Acquired(attempt)) => Some(attempt),
            Ok(AsyncJobClaim::Busy { .. }) | Err(_) => None,
        }
    }

    /// Clear an expired attempt after its authoritative domain proves no work remains.
    pub fn abandon_expired(owner: AsyncJobOwner, now_ns: u64) -> bool {
        if AsyncJobRecoveryOps::expired_deadline(owner, now_ns).is_none() {
            return false;
        }
        AsyncJobRecoveryOps::abandon(owner);
        true
    }

    /// Finish only the exact active attempt and preserve the provider result when current.
    pub fn finish(attempt: AsyncJobAttempt, result: TimerRunResult) -> TimerRunResult {
        let completion = async_job_completion(result.completion().outcome());
        match AsyncJobRecoveryOps::finish(attempt, completion) {
            Ok(true) => result,
            Ok(false) => TimerRunResult::new(TimerCompletion::no_work(), TimerDirective::Stop),
            Err(_) => invariant_failure(),
        }
    }
}

const fn async_job_completion(outcome: TimerCompletionOutcome) -> AsyncJobCompletion {
    match outcome {
        TimerCompletionOutcome::Success | TimerCompletionOutcome::NoWork => {
            AsyncJobCompletion::Success
        }
        TimerCompletionOutcome::RetryableFailure => AsyncJobCompletion::RetryableFailure,
        TimerCompletionOutcome::InvariantFailure => AsyncJobCompletion::InvariantFailure,
    }
}

const fn invariant_failure() -> TimerRunResult {
    TimerRunResult::new(TimerCompletion::invariant_failure(0), TimerDirective::Stop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::stable::async_job_recovery::{AsyncJobRecoveryData, AsyncJobRecoveryStore};

    #[test]
    fn expired_abandonment_clears_only_expired_domain_work() {
        AsyncJobRecoveryStore::import(AsyncJobRecoveryData::default());
        let owner = AsyncJobOwner::AuthRenewal;
        assert!(matches!(
            AsyncJobRecoveryOps::claim(owner, 10, 20),
            Ok(AsyncJobClaim::Acquired(_))
        ));
        assert!(AsyncJobWorkflow::has_active_attempt(owner));

        assert!(!AsyncJobWorkflow::abandon_expired(owner, 19));
        assert!(!AsyncJobWorkflow::has_expired_attempt(owner, 19));
        assert!(AsyncJobWorkflow::has_expired_attempt(owner, 20));
        assert!(AsyncJobWorkflow::abandon_expired(owner, 20));
        assert!(!AsyncJobWorkflow::has_active_attempt(owner));
        assert_eq!(AsyncJobRecoveryOps::active_lease_deadline(owner), None);
    }
}
