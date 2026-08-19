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
    assert_eq!(first.operation_id(), None);
    assert_eq!(takeover.operation_id(), None);
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
    assert_eq!(first.operation_id(), takeover.operation_id());

    assert!(
        AsyncJobRecoveryOps::finish(takeover, AsyncJobCompletion::RetryableFailure)
            .expect("finish retryable operation")
    );
    let retry = acquired(AsyncJobRecoveryOps::claim(owner, 41, 50).expect("claim exact retry"));
    assert_eq!(takeover.operation_id(), retry.operation_id());

    assert!(
        AsyncJobRecoveryOps::finish(retry, AsyncJobCompletion::Success)
            .expect("finish exact retry")
    );
    let next = acquired(AsyncJobRecoveryOps::claim(owner, 51, 60).expect("claim next operation"));
    assert_ne!(retry.operation_id(), next.operation_id());
}

#[test]
fn non_cycle_retry_completion_retains_no_generated_operation_identity() {
    let _guard = crate::test::seams::lock();
    reset();
    let owner = AsyncJobOwner::PlacementReceiptAcknowledgement;
    let first = acquired(AsyncJobRecoveryOps::claim(owner, 1, 3).expect("claim placement job"));
    assert!(
        AsyncJobRecoveryOps::finish(first, AsyncJobCompletion::RetryableFailure)
            .expect("finish placement job")
    );
    let retry = acquired(AsyncJobRecoveryOps::claim(owner, 4, 6).expect("claim next attempt"));
    assert_eq!(first.operation_id(), None);
    assert_eq!(retry.operation_id(), None);
}

#[test]
fn stale_completion_cannot_clear_a_takeover_attempt() {
    let _guard = crate::test::seams::lock();
    reset();
    let owner = AsyncJobOwner::CanisterPoolMaintenance;
    let first = acquired(AsyncJobRecoveryOps::claim(owner, 1, 2).expect("claim first attempt"));
    let takeover = acquired(AsyncJobRecoveryOps::claim(owner, 2, 4).expect("take over attempt"));

    assert!(
        !AsyncJobRecoveryOps::finish(first, AsyncJobCompletion::Success)
            .expect("reject stale finish")
    );
    assert_eq!(
        AsyncJobRecoveryOps::claim(owner, 3, 5).expect("observe takeover"),
        AsyncJobClaim::Busy { retry_at_ns: 4 }
    );
    assert!(
        AsyncJobRecoveryOps::finish(takeover, AsyncJobCompletion::Success)
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
        AsyncJobRecoveryOps::finish(cycle, AsyncJobCompletion::RetryableFailure)
            .expect("retain cycle retry")
    );
    AsyncJobRecoveryOps::abandon(AsyncJobOwner::CycleTopup);
    let next = acquired(
        AsyncJobRecoveryOps::claim(AsyncJobOwner::CycleTopup, 3, 4).expect("claim after abandon"),
    );
    assert_ne!(cycle.operation_id(), next.operation_id());

    let pool = acquired(
        AsyncJobRecoveryOps::claim(AsyncJobOwner::CanisterPoolMaintenance, 5, 6)
            .expect("claim pool attempt"),
    );
    AsyncJobRecoveryOps::abandon(AsyncJobOwner::CanisterPoolMaintenance);
    assert_eq!(
        AsyncJobRecoveryOps::expired_deadline(AsyncJobOwner::CanisterPoolMaintenance, u64::MAX),
        None
    );
    assert_eq!(pool.operation_id(), None);
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
        let first_operation_id = first.operation_id();
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
            !AsyncJobRecoveryOps::finish(first, AsyncJobCompletion::Success)
                .expect("reject a late completion from the lost response")
        );
        assert_eq!(AsyncJobRecoveryOps::active_lease_deadline(owner), Some(40));

        if owner == AsyncJobOwner::CycleTopup {
            assert_eq!(takeover.operation_id(), first_operation_id);
            assert!(
                AsyncJobRecoveryOps::finish(takeover, AsyncJobCompletion::RetryableFailure)
                    .expect("retain exact uncertain funding identity")
            );
            let retry = acquired(
                AsyncJobRecoveryOps::claim(owner, 41, 60)
                    .expect("retry the same parent-funding operation"),
            );
            assert_eq!(retry.operation_id(), first_operation_id);
            assert!(
                AsyncJobRecoveryOps::finish(retry, AsyncJobCompletion::Success)
                    .expect("commit the exact funding retry")
            );
        } else {
            assert_eq!(first_operation_id, None);
            assert_eq!(takeover.operation_id(), None);
            assert!(
                AsyncJobRecoveryOps::finish(takeover, AsyncJobCompletion::Success)
                    .expect("commit the owner-bound domain operation")
            );
        }

        assert_eq!(AsyncJobRecoveryOps::active_lease_deadline(owner), None);
    }
}
