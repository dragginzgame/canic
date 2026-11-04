//! Module: ops::storage::async_job_recovery::fixture_import
//!
//! Responsibility: commit bounded import attempt custody before callback execution.
//! Does not own: application checkpoints, callback rollback or timer registration.
//! Boundary: only the exact active attempt may change its retained position.

use crate::{
    InternalError,
    domain::fixture_import::{FixtureImportFailure, FixtureImportRecoveryStatus},
    ops::storage::async_job_recovery::{AsyncJobAttempt, AsyncJobRecoveryOps},
    storage::stable::async_job_recovery::{
        AsyncJobRecoveryStore, FixtureImportRecoveryRecord, FixtureImportRetryRecord,
    },
};

const FIXTURE_IMPORT_ATTEMPT_LIMIT: u8 = 3;

impl AsyncJobRecoveryOps {
    /// Charge before dispatch; an interrupted callback cannot roll back this message.
    pub fn admit_fixture_work(
        attempt: AsyncJobAttempt,
        assignment_hash: [u8; 32],
    ) -> Result<bool, InternalError> {
        if !Self::is_current(attempt) {
            return Err(InternalError::conflict());
        }
        let mut state = AsyncJobRecoveryStore::get();
        let recovery = state
            .fixture_import_recovery
            .get_or_insert(FixtureImportRecoveryRecord {
                assignment_hash,
                next_chunk: None,
                unfinished_attempts: 0,
            });
        let failure = if recovery.assignment_hash != assignment_hash {
            Some(FixtureImportFailure::Authority)
        } else if recovery.unfinished_attempts >= FIXTURE_IMPORT_ATTEMPT_LIMIT {
            Some(FixtureImportFailure::Interrupted {
                next_chunk: recovery.next_chunk,
                attempts: recovery.unfinished_attempts,
            })
        } else {
            None
        };
        if let Some(failure) = failure {
            state.fixture_import_failure = Some(failure);
            state.fixture_import.active = None;
            AsyncJobRecoveryStore::replace(state);
            return Ok(false);
        }
        recovery.unfinished_attempts += 1;
        AsyncJobRecoveryStore::replace(state);
        Ok(true)
    }

    /// Record only successfully validated application progress, in its commit message.
    pub fn record_fixture_position(
        attempt: AsyncJobAttempt,
        next_chunk: u32,
    ) -> Result<(), InternalError> {
        if !Self::is_current(attempt) {
            return Err(InternalError::conflict());
        }
        let mut state = AsyncJobRecoveryStore::get();
        let recovery = state
            .fixture_import_recovery
            .as_mut()
            .ok_or_else(InternalError::invariant)?;
        recovery.next_chunk = Some(next_chunk);
        AsyncJobRecoveryStore::replace(state);
        Ok(())
    }

    /// Expose custody without replacing the application's authoritative checkpoint.
    #[must_use]
    pub fn fixture_import_recovery() -> Option<FixtureImportRecoveryStatus> {
        let state = AsyncJobRecoveryStore::get();
        state
            .fixture_import_recovery
            .map(|recovery| FixtureImportRecoveryStatus {
                assignment_hash: recovery.assignment_hash,
                next_chunk: recovery.next_chunk,
                unfinished_attempts: recovery.unfinished_attempts,
                attempt_limit: FIXTURE_IMPORT_ATTEMPT_LIMIT,
                active: state.fixture_import.active.is_some(),
            })
    }

    /// Deliberately reopen failed work under the same installed assignment only.
    pub fn require_fixture_retry(assignment_hash: [u8; 32]) -> Result<(), InternalError> {
        let state = AsyncJobRecoveryStore::get();
        if state.fixture_import.active.is_some() || state.fixture_import_failure.is_none() {
            return Err(InternalError::conflict());
        }
        if state
            .fixture_import_recovery
            .as_ref()
            .is_none_or(|recovery| recovery.assignment_hash != assignment_hash)
        {
            return Err(InternalError::conflict());
        }
        Ok(())
    }

    /// Commit an explicitly reviewed retry after timer admission succeeds.
    pub fn retry_fixture_import(assignment_hash: [u8; 32]) -> Result<(), InternalError> {
        Self::require_fixture_retry(assignment_hash)?;
        let mut state = AsyncJobRecoveryStore::get();
        let recovery = state
            .fixture_import_recovery
            .as_mut()
            .ok_or_else(InternalError::invariant)?;
        recovery.unfinished_attempts = 0;
        state.fixture_import_failure = None;
        state.fixture_import_retry = FixtureImportRetryRecord::default();
        AsyncJobRecoveryStore::replace(state);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::storage::async_job_recovery::{
        AsyncJobClaim, AsyncJobCompletion, AsyncJobOwner,
    };

    fn claim(now: u64) -> AsyncJobAttempt {
        let AsyncJobClaim::Acquired(attempt) =
            AsyncJobRecoveryOps::claim(AsyncJobOwner::FixtureImport, now, now + 10).unwrap()
        else {
            panic!("free attempt");
        };
        attempt
    }

    #[test]
    fn interrupted_fixture_work_stops_at_one_position_and_requires_exact_retry() {
        AsyncJobRecoveryOps::reset_for_tests();
        let initial = claim(0);
        assert!(AsyncJobRecoveryOps::admit_fixture_work(initial, [1; 32]).unwrap());
        AsyncJobRecoveryOps::record_fixture_position(initial, 7).unwrap();
        assert!(AsyncJobRecoveryOps::finish(initial, AsyncJobCompletion::Success, 1).unwrap());
        for count in 1..=FIXTURE_IMPORT_ATTEMPT_LIMIT {
            let attempt = claim(u64::from(count) * 20);
            assert!(AsyncJobRecoveryOps::admit_fixture_work(attempt, [1; 32]).unwrap());
            let recovery = AsyncJobRecoveryOps::fixture_import_recovery().unwrap();
            assert_eq!(recovery.next_chunk, Some(7));
            assert_eq!(recovery.unfinished_attempts, count);
            assert!(AsyncJobRecoveryOps::retry_fixture_import([1; 32]).is_err());
            AsyncJobRecoveryOps::abandon(AsyncJobOwner::FixtureImport);
        }
        let final_attempt = claim(100);
        assert!(!AsyncJobRecoveryOps::admit_fixture_work(final_attempt, [1; 32]).unwrap());
        assert_eq!(
            AsyncJobRecoveryOps::fixture_import_failure(),
            Some(FixtureImportFailure::Interrupted {
                next_chunk: Some(7),
                attempts: FIXTURE_IMPORT_ATTEMPT_LIMIT
            })
        );
        let failed = AsyncJobRecoveryStore::export();
        assert!(AsyncJobRecoveryOps::retry_fixture_import([2; 32]).is_err());
        assert_eq!(AsyncJobRecoveryStore::export(), failed);
        AsyncJobRecoveryOps::retry_fixture_import([1; 32]).unwrap();
        assert_eq!(AsyncJobRecoveryOps::fixture_import_failure(), None);
        assert_eq!(
            AsyncJobRecoveryOps::fixture_import_recovery()
                .unwrap()
                .next_chunk,
            Some(7)
        );
        let resumed = claim(120);
        assert!(AsyncJobRecoveryOps::admit_fixture_work(resumed, [1; 32]).unwrap());
        AsyncJobRecoveryOps::record_fixture_position(resumed, 8).unwrap();
        AsyncJobRecoveryOps::finish(resumed, AsyncJobCompletion::Success, 121).unwrap();
        assert_eq!(
            AsyncJobRecoveryOps::fixture_import_recovery()
                .unwrap()
                .unfinished_attempts,
            0
        );
    }

    #[test]
    fn source_outage_does_not_charge_the_callback_budget_or_erase_prior_interruptions() {
        AsyncJobRecoveryOps::reset_for_tests();
        let interrupted = claim(0);
        assert!(AsyncJobRecoveryOps::admit_fixture_work(interrupted, [1; 32]).unwrap());
        AsyncJobRecoveryOps::abandon(AsyncJobOwner::FixtureImport);
        let source = claim(20);
        assert!(AsyncJobRecoveryOps::admit_fixture_work(source, [1; 32]).unwrap());
        assert!(
            AsyncJobRecoveryOps::finish(source, AsyncJobCompletion::RetryableFailure, 21).unwrap()
        );
        let after = AsyncJobRecoveryOps::fixture_import_recovery().unwrap();
        assert_eq!(after.unfinished_attempts, 1);
        assert!(!after.active);
        let unchanged = AsyncJobRecoveryStore::export();
        assert!(AsyncJobRecoveryOps::record_fixture_position(interrupted, 99).is_err());
        assert!(AsyncJobRecoveryOps::admit_fixture_work(interrupted, [1; 32]).is_err());
        assert_eq!(AsyncJobRecoveryStore::export(), unchanged);
    }

    #[test]
    fn changed_assignment_cannot_reset_interrupted_work() {
        AsyncJobRecoveryOps::reset_for_tests();
        let original = claim(0);
        assert!(AsyncJobRecoveryOps::admit_fixture_work(original, [1; 32]).unwrap());
        AsyncJobRecoveryOps::abandon(AsyncJobOwner::FixtureImport);
        let changed = claim(20);
        assert!(!AsyncJobRecoveryOps::admit_fixture_work(changed, [2; 32]).unwrap());
        assert_eq!(
            AsyncJobRecoveryOps::fixture_import_failure(),
            Some(FixtureImportFailure::Authority)
        );
        assert_eq!(
            AsyncJobRecoveryOps::fixture_import_recovery()
                .unwrap()
                .assignment_hash,
            [1; 32]
        );
    }
}
