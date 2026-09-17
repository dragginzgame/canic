use super::*;
use crate::storage::stable::async_job_recovery::{
    AsyncAttemptFenceRecord, AsyncJobRecoveryData, AsyncJobRecoveryStore,
    ReplaySafeAsyncAttemptFenceRecord,
};

fn reset() {
    AsyncJobRecoveryStore::import(AsyncJobRecoveryData::default());
}

fn acquired(claim: AsyncJobClaim) -> AsyncJobAttempt {
    match claim {
        AsyncJobClaim::Acquired(attempt) => attempt,
        AsyncJobClaim::Busy { .. } => panic!("expected acquired async-job attempt"),
    }
}

#[test]
fn live_attempts_coalesce_and_expired_minimal_takeover_advances_only_the_attempt() {
    let _guard = crate::test::seams::lock();
    reset();
    let owner = AsyncJobOwner::AuthRenewal;
    let first = acquired(AsyncJobRecoveryOps::claim(owner, 10, 20).expect("claim first attempt"));
    assert_eq!(
        AsyncJobRecoveryOps::claim(owner, 19, 30).expect("coalesce live attempt"),
        AsyncJobClaim::Busy { retry_at_ns: 20 }
    );

    let takeover = acquired(AsyncJobRecoveryOps::claim(owner, 20, 40).expect("take over attempt"));
    assert_ne!(first.attempt_generation, takeover.attempt_generation);
    assert_eq!(first.operation_id(crate::test::seams::p(1)), None);
    assert_eq!(takeover.operation_id(crate::test::seams::p(1)), None);
}

#[test]
fn cycle_takeover_and_retry_reuse_only_the_exact_cycle_operation() {
    let _guard = crate::test::seams::lock();
    reset();
    let owner = AsyncJobOwner::CycleTopup;
    let first = acquired(AsyncJobRecoveryOps::claim(owner, 10, 20).expect("claim first operation"));
    let takeover =
        acquired(AsyncJobRecoveryOps::claim(owner, 20, 40).expect("take over operation"));
    assert_ne!(first.attempt_generation, takeover.attempt_generation);
    assert_eq!(
        first.operation_id(crate::test::seams::p(1)),
        takeover.operation_id(crate::test::seams::p(1))
    );

    assert!(
        AsyncJobRecoveryOps::finish(takeover, AsyncJobCompletion::RetryableFailure, 0)
            .expect("finish retryable operation")
    );
    let retry = acquired(AsyncJobRecoveryOps::claim(owner, 41, 50).expect("claim exact retry"));
    assert_eq!(
        takeover.operation_id(crate::test::seams::p(1)),
        retry.operation_id(crate::test::seams::p(1))
    );

    assert!(
        AsyncJobRecoveryOps::finish(retry, AsyncJobCompletion::Success, 0)
            .expect("finish exact retry")
    );
    let next = acquired(AsyncJobRecoveryOps::claim(owner, 51, 60).expect("claim next operation"));
    assert_ne!(
        retry.operation_id(crate::test::seams::p(1)),
        next.operation_id(crate::test::seams::p(1))
    );
}

#[test]
fn non_cycle_retry_completion_retains_no_generated_operation_identity() {
    let _guard = crate::test::seams::lock();
    reset();
    let owner = AsyncJobOwner::PlacementReceiptAcknowledgement;
    let first = acquired(AsyncJobRecoveryOps::claim(owner, 1, 3).expect("claim placement job"));
    assert!(
        AsyncJobRecoveryOps::finish(first, AsyncJobCompletion::RetryableFailure, 0)
            .expect("finish placement job")
    );
    let retry = acquired(AsyncJobRecoveryOps::claim(owner, 4, 6).expect("claim next attempt"));
    assert_eq!(first.operation_id(crate::test::seams::p(1)), None);
    assert_eq!(retry.operation_id(crate::test::seams::p(1)), None);
}

#[test]
fn stale_completion_cannot_clear_a_takeover_attempt() {
    let _guard = crate::test::seams::lock();
    reset();
    let owner = AsyncJobOwner::CanisterPoolMaintenance;
    let first = acquired(AsyncJobRecoveryOps::claim(owner, 1, 2).expect("claim first attempt"));
    let takeover = acquired(AsyncJobRecoveryOps::claim(owner, 2, 4).expect("take over attempt"));

    assert!(
        !AsyncJobRecoveryOps::finish(first, AsyncJobCompletion::Success, 0)
            .expect("reject stale finish")
    );
    assert_eq!(
        AsyncJobRecoveryOps::claim(owner, 3, 5).expect("observe takeover"),
        AsyncJobClaim::Busy { retry_at_ns: 4 }
    );
    assert!(
        AsyncJobRecoveryOps::finish(takeover, AsyncJobCompletion::Success, 0)
            .expect("finish takeover")
    );
}

#[test]
fn abandon_clears_only_active_and_cycle_retry_authority() {
    let _guard = crate::test::seams::lock();
    reset();
    let cycle = acquired(
        AsyncJobRecoveryOps::claim(AsyncJobOwner::CycleTopup, 1, 2).expect("claim cycle operation"),
    );
    assert!(
        AsyncJobRecoveryOps::finish(cycle, AsyncJobCompletion::RetryableFailure, 0)
            .expect("retain cycle retry")
    );
    AsyncJobRecoveryOps::abandon(AsyncJobOwner::CycleTopup);
    let next = acquired(
        AsyncJobRecoveryOps::claim(AsyncJobOwner::CycleTopup, 3, 4).expect("claim after abandon"),
    );
    assert_ne!(
        cycle.operation_id(crate::test::seams::p(1)),
        next.operation_id(crate::test::seams::p(1))
    );

    let pool = acquired(
        AsyncJobRecoveryOps::claim(AsyncJobOwner::CanisterPoolMaintenance, 5, 6)
            .expect("claim pool attempt"),
    );
    AsyncJobRecoveryOps::abandon(AsyncJobOwner::CanisterPoolMaintenance);
    assert_eq!(
        AsyncJobRecoveryOps::expired_deadline(AsyncJobOwner::CanisterPoolMaintenance, u64::MAX),
        None
    );
    assert_eq!(pool.operation_id(crate::test::seams::p(1)), None);
}

#[test]
fn invalid_lease_and_generation_exhaustion_fail_without_mutation() {
    let _guard = crate::test::seams::lock();
    reset();
    assert!(AsyncJobRecoveryOps::claim(AsyncJobOwner::AuthRenewal, 2, 2).is_err());

    let mut record = AsyncJobRecoveryStore::export().record;
    record.auth_renewal = AsyncAttemptFenceRecord {
        last_attempt_generation: u64::MAX,
        active: None,
    };
    record.cycle_topup = ReplaySafeAsyncAttemptFenceRecord {
        last_attempt_generation: 0,
        last_operation_generation: u64::MAX,
        active: None,
        pending_operation_generation: None,
    };
    AsyncJobRecoveryStore::import(AsyncJobRecoveryData {
        record: record.clone(),
    });

    assert!(AsyncJobRecoveryOps::claim(AsyncJobOwner::AuthRenewal, 1, 2).is_err());
    assert!(AsyncJobRecoveryOps::claim(AsyncJobOwner::CycleTopup, 1, 2).is_err());
    assert_eq!(AsyncJobRecoveryStore::export().record, record);
}

#[test]
fn every_closed_owner_survives_response_loss_restart_and_one_fenced_takeover() {
    let _guard = crate::test::seams::lock();
    let journeys = [
        (AsyncJobOwner::AuthRenewal, "issuer-template-and-proof"),
        (
            AsyncJobOwner::CanisterPoolMaintenance,
            "pool-record-and-maintenance-journal",
        ),
        (AsyncJobOwner::CycleTopup, "parent-funding-operation"),
        (
            AsyncJobOwner::FixtureImport,
            "installed-fixture-and-application-receipt",
        ),
        (
            AsyncJobOwner::PlacementReceiptAcknowledgement,
            "terminal-placement-receipt",
        ),
    ];

    for (owner, authoritative_domain_identity) in journeys {
        reset();
        let first = acquired(
            AsyncJobRecoveryOps::claim(owner, 10, 20)
                .expect("record demand and claim the first external-effect attempt"),
        );
        let first_operation_id = first.operation_id(crate::test::seams::p(1));
        let committed_boundary = AsyncJobRecoveryStore::export();

        // Simulate a lost response and same-release heap restart. Domain demand lives in
        // its owner; memory ID 60 restores only the exact attempt fence.
        AsyncJobRecoveryStore::import(committed_boundary);
        assert!(!authoritative_domain_identity.is_empty());
        assert_eq!(
            AsyncJobRecoveryOps::claim(owner, 19, 30).expect("reject overlap before lease expiry"),
            AsyncJobClaim::Busy { retry_at_ns: 20 }
        );

        let takeover = acquired(
            AsyncJobRecoveryOps::claim(owner, 20, 40)
                .expect("claim the single fenced takeover at lease expiry"),
        );
        assert_eq!(takeover.attempt_generation, first.attempt_generation + 1);
        assert_eq!(
            AsyncJobRecoveryOps::claim(owner, 21, 50).expect("coalesce every competing takeover"),
            AsyncJobClaim::Busy { retry_at_ns: 40 }
        );
        assert!(
            !AsyncJobRecoveryOps::finish(first, AsyncJobCompletion::Success, 0)
                .expect("reject a late completion from the lost response")
        );
        assert_eq!(AsyncJobRecoveryOps::active_lease_deadline(owner), Some(40));

        if owner == AsyncJobOwner::CycleTopup {
            assert_eq!(
                takeover.operation_id(crate::test::seams::p(1)),
                first_operation_id
            );
            assert!(
                AsyncJobRecoveryOps::finish(takeover, AsyncJobCompletion::RetryableFailure, 0)
                    .expect("retain exact uncertain funding identity")
            );
            let retry = acquired(
                AsyncJobRecoveryOps::claim(owner, 41, 60)
                    .expect("retry the same parent-funding operation"),
            );
            assert_eq!(
                retry.operation_id(crate::test::seams::p(1)),
                first_operation_id
            );
            assert!(
                AsyncJobRecoveryOps::finish(retry, AsyncJobCompletion::Success, 0)
                    .expect("commit the exact funding retry")
            );
        } else {
            assert_eq!(first_operation_id, None);
            assert_eq!(takeover.operation_id(crate::test::seams::p(1)), None);
            assert!(
                AsyncJobRecoveryOps::finish(takeover, AsyncJobCompletion::Success, 0)
                    .expect("commit the owner-bound domain operation")
            );
        }

        assert_eq!(AsyncJobRecoveryOps::active_lease_deadline(owner), None);
    }
}

#[test]
fn permanent_fixture_failure_survives_restore_and_stale_attempt_cannot_replace_it() {
    use crate::domain::fixture_import::FixtureImportFailure;
    let _guard = crate::test::seams::lock();
    reset();
    let first = acquired(AsyncJobRecoveryOps::claim(AsyncJobOwner::FixtureImport, 1, 2).unwrap());
    let next = acquired(AsyncJobRecoveryOps::claim(AsyncJobOwner::FixtureImport, 2, 4).unwrap());
    assert!(!AsyncJobRecoveryOps::is_current(first));
    assert!(AsyncJobRecoveryOps::is_current(next));
    assert!(!AsyncJobRecoveryOps::fail_fixture_import(
        first,
        FixtureImportFailure::Authority
    ));
    assert_eq!(AsyncJobRecoveryOps::fixture_import_failure(), None);
    let failure = FixtureImportFailure::Application { code: 42 };
    assert!(AsyncJobRecoveryOps::fail_fixture_import(next, failure));
    let snapshot = AsyncJobRecoveryStore::export();
    AsyncJobRecoveryStore::import(snapshot);
    assert_eq!(AsyncJobRecoveryOps::fixture_import_failure(), Some(failure));
    assert!(!AsyncJobRecoveryOps::is_current(next));
}

#[test]
fn fixture_backoff_survives_restore_and_admits_only_one_retry_at_the_deadline() {
    let _guard = crate::test::seams::lock();
    reset();
    let owner = AsyncJobOwner::FixtureImport;
    let mut now = 1;
    for seconds in [1, 2, 4, 8, 16, 32, 60, 60] {
        let attempt = acquired(AsyncJobRecoveryOps::claim(owner, now, now + 100).unwrap());
        assert!(
            AsyncJobRecoveryOps::finish(attempt, AsyncJobCompletion::RetryableFailure, now)
                .unwrap()
        );
        let deadline = now + seconds * 1_000_000_000;
        let retained = AsyncJobRecoveryStore::export();
        assert_eq!(retained.record.fixture_import_retry.not_before_ns, deadline);
        AsyncJobRecoveryStore::import(retained.clone());
        assert_eq!(
            AsyncJobRecoveryOps::claim(owner, deadline - 1, deadline + 100).unwrap(),
            AsyncJobClaim::Busy {
                retry_at_ns: deadline
            }
        );
        assert_eq!(AsyncJobRecoveryStore::export(), retained);
        assert!(
            !AsyncJobRecoveryOps::finish(attempt, AsyncJobCompletion::Success, deadline - 1)
                .unwrap()
        );
        assert_eq!(AsyncJobRecoveryStore::export(), retained);
        now = deadline;
    }
    let retry = acquired(AsyncJobRecoveryOps::claim(owner, now, now + 100).unwrap());
    assert_eq!(
        AsyncJobRecoveryOps::claim(owner, now, now + 200).unwrap(),
        AsyncJobClaim::Busy {
            retry_at_ns: now + 100
        }
    );
    assert!(AsyncJobRecoveryOps::finish(retry, AsyncJobCompletion::Success, now).unwrap());
    assert_eq!(
        AsyncJobRecoveryStore::export().record.fixture_import_retry,
        FixtureImportRetryRecord::default()
    );
    let progress = acquired(AsyncJobRecoveryOps::claim(owner, now, now + 100).unwrap());
    assert!(
        AsyncJobRecoveryOps::finish(progress, AsyncJobCompletion::RetryableFailure, now).unwrap()
    );
    assert_eq!(
        AsyncJobRecoveryStore::export()
            .record
            .fixture_import_retry
            .not_before_ns,
        now + 1_000_000_000
    );
}

#[test]
fn fixture_retry_fences_stale_and_foreign_completions_and_saturates_without_early_retry() {
    let _guard = crate::test::seams::lock();
    reset();
    let owner = AsyncJobOwner::FixtureImport;
    let stale = acquired(AsyncJobRecoveryOps::claim(owner, 1, 2).unwrap());
    let current = acquired(AsyncJobRecoveryOps::claim(owner, 2, 4).unwrap());
    let foreign = acquired(AsyncJobRecoveryOps::claim(AsyncJobOwner::AuthRenewal, 2, 4).unwrap());
    let retained = AsyncJobRecoveryStore::export();
    assert!(!AsyncJobRecoveryOps::finish(stale, AsyncJobCompletion::RetryableFailure, 3).unwrap());
    assert_eq!(AsyncJobRecoveryStore::export(), retained);
    assert!(AsyncJobRecoveryOps::finish(foreign, AsyncJobCompletion::RetryableFailure, 3).unwrap());
    assert_eq!(
        AsyncJobRecoveryStore::export().record.fixture_import_retry,
        retained.record.fixture_import_retry
    );
    let mut saturated = AsyncJobRecoveryStore::export();
    saturated.record.fixture_import_retry.failures = u32::MAX;
    AsyncJobRecoveryStore::import(saturated);
    assert!(
        AsyncJobRecoveryOps::finish(current, AsyncJobCompletion::RetryableFailure, u64::MAX - 1)
            .unwrap()
    );
    assert_eq!(
        AsyncJobRecoveryStore::export()
            .record
            .fixture_import_retry
            .failures,
        u32::MAX
    );
    assert_eq!(
        AsyncJobRecoveryOps::claim(owner, u64::MAX - 1, u64::MAX).unwrap(),
        AsyncJobClaim::Busy {
            retry_at_ns: u64::MAX
        }
    );
    // The fixture's outage delay does not block other recovery owners.
    let _ = acquired(AsyncJobRecoveryOps::claim(AsyncJobOwner::AuthRenewal, 3, 5).unwrap());
}

#[test]
fn sibling_cycle_generations_have_independent_receipts_across_retry_and_restoration() {
    let _guard = crate::test::seams::lock();
    reset();
    let owner = AsyncJobOwner::CycleTopup;
    let first = acquired(AsyncJobRecoveryOps::claim(owner, 10, 20).unwrap());
    let left = crate::test::seams::p(1);
    let right = crate::test::seams::p(2);
    let left_id = first.operation_id(left).unwrap();
    let right_id = first.operation_id(right).unwrap();
    assert_ne!(left_id, right_id);
    // The same stable generation on a fresh heap preserves each caller's identity.
    let snapshot = AsyncJobRecoveryStore::export();
    reset();
    AsyncJobRecoveryStore::import(snapshot);
    let takeover = acquired(AsyncJobRecoveryOps::claim(owner, 20, 30).unwrap());
    assert_eq!(takeover.operation_id(left), Some(left_id));
    assert_eq!(takeover.operation_id(right), Some(right_id));
    assert!(
        AsyncJobRecoveryOps::finish(takeover, AsyncJobCompletion::RetryableFailure, 25).unwrap()
    );
    let retry = acquired(AsyncJobRecoveryOps::claim(owner, 31, 40).unwrap());
    assert_eq!(retry.operation_id(left), Some(left_id));
    assert_eq!(retry.operation_id(right), Some(right_id));
    assert!(AsyncJobRecoveryOps::finish(retry, AsyncJobCompletion::Success, 35).unwrap());
    let next = acquired(AsyncJobRecoveryOps::claim(owner, 41, 50).unwrap());
    assert_ne!(next.operation_id(left), Some(left_id));
    assert_ne!(next.operation_id(right), Some(right_id));
    assert_ne!(next.operation_id(left), next.operation_id(right));
}
