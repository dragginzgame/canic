use super::*;
use crate::storage::stable::async_recovery::{
    AsyncRecoveryOwnerRecord, AsyncTimerRecoveryData, AsyncTimerRecoveryStore,
};

fn reset() {
    AsyncTimerRecoveryStore::import(AsyncTimerRecoveryData::default());
}

fn acquired(claim: AsyncRecoveryClaim) -> AsyncRecoveryAttempt {
    match claim {
        AsyncRecoveryClaim::Acquired(attempt) => attempt,
        AsyncRecoveryClaim::Busy { .. } => panic!("expected acquired recovery attempt"),
    }
}

#[test]
fn live_attempts_coalesce_and_expired_takeover_reuses_the_operation() {
    reset();
    let first = acquired(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::CycleTopup, 10, 20)
            .expect("claim first attempt"),
    );
    assert_eq!(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::CycleTopup, 19, 30)
            .expect("coalesce live attempt"),
        AsyncRecoveryClaim::Busy { retry_at_ns: 20 }
    );

    let takeover = acquired(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::CycleTopup, 20, 40)
            .expect("take over expired attempt"),
    );
    assert_ne!(first.attempt_generation, takeover.attempt_generation);
    assert_eq!(first.operation_id(), takeover.operation_id());
}

#[test]
fn stale_completion_cannot_clear_a_takeover_attempt() {
    reset();
    let first = acquired(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::AuthRenewal, 1, 2)
            .expect("claim first attempt"),
    );
    let takeover = acquired(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::AuthRenewal, 2, 4)
            .expect("take over expired attempt"),
    );

    assert!(
        !AsyncTimerRecoveryOps::finish(first, AsyncRecoveryCompletion::Success, None)
            .expect("reject stale finish")
    );
    assert_eq!(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::AuthRenewal, 3, 5)
            .expect("observe takeover"),
        AsyncRecoveryClaim::Busy { retry_at_ns: 4 }
    );
    assert!(
        AsyncTimerRecoveryOps::finish(takeover, AsyncRecoveryCompletion::Success, None)
            .expect("finish takeover")
    );
}

#[test]
fn retry_completion_reuses_operation_but_terminal_completion_advances_it() {
    reset();
    let first = acquired(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::PlacementReceiptAcknowledgement, 1, 3)
            .expect("claim first operation"),
    );
    assert!(
        AsyncTimerRecoveryOps::finish(first, AsyncRecoveryCompletion::RetryableFailure, None)
            .expect("retain retry operation")
    );
    let retry = acquired(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::PlacementReceiptAcknowledgement, 2, 4)
            .expect("claim exact retry"),
    );
    assert_eq!(first.operation_id(), retry.operation_id());
    assert!(
        AsyncTimerRecoveryOps::finish(retry, AsyncRecoveryCompletion::Success, None)
            .expect("finish operation")
    );

    let next = acquired(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::PlacementReceiptAcknowledgement, 3, 5)
            .expect("claim next operation"),
    );
    assert_ne!(first.operation_id(), next.operation_id());
}

#[test]
fn abandon_clears_expired_and_pending_recovery_demand() {
    reset();
    let attempt = acquired(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::CanisterPoolMaintenance, 1, 2)
            .expect("claim maintenance"),
    );
    assert!(
        AsyncTimerRecoveryOps::finish(attempt, AsyncRecoveryCompletion::RetryableFailure, None,)
            .expect("retain pending operation")
    );
    let active = acquired(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::CanisterPoolMaintenance, 2, 3)
            .expect("claim retry"),
    );
    assert_eq!(attempt.operation_id(), active.operation_id());

    AsyncTimerRecoveryOps::abandon(AsyncRecoveryOwner::CanisterPoolMaintenance);
    assert_eq!(
        AsyncTimerRecoveryOps::expired_deadline(
            AsyncRecoveryOwner::CanisterPoolMaintenance,
            u64::MAX
        ),
        None
    );
    let next = acquired(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::CanisterPoolMaintenance, 4, 5)
            .expect("claim after abandon"),
    );
    assert_ne!(attempt.operation_id(), next.operation_id());
}

#[test]
fn takeover_schedule_and_domain_outcomes_remain_durable() {
    reset();
    AsyncTimerRecoveryOps::activate_recovery(AsyncRecoveryOwner::AuthRenewal, 10);
    assert_eq!(
        AsyncTimerRecoveryOps::recovery_due(AsyncRecoveryOwner::AuthRenewal, 9),
        None
    );
    assert_eq!(
        AsyncTimerRecoveryOps::recovery_due(AsyncRecoveryOwner::AuthRenewal, 10),
        Some(10)
    );
    let attempt = acquired(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::AuthRenewal, 10, 20)
            .expect("claim watchdog-owned attempt"),
    );
    assert!(
        AsyncTimerRecoveryOps::finish(
            attempt,
            AsyncRecoveryCompletion::RetryableFailure,
            Some(30),
        )
        .expect("finish retryable watchdog attempt")
    );
    assert_eq!(
        AsyncTimerRecoveryOps::retry_streak(AsyncRecoveryOwner::AuthRenewal),
        1
    );
    assert_eq!(
        AsyncTimerRecoveryOps::recovery_due(AsyncRecoveryOwner::AuthRenewal, 30),
        Some(30)
    );

    let retry = acquired(
        AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::AuthRenewal, 30, 40).expect("claim retry"),
    );
    assert!(
        AsyncTimerRecoveryOps::finish(retry, AsyncRecoveryCompletion::InvariantFailure, None,)
            .expect("finish invariant attempt")
    );
    assert!(AsyncTimerRecoveryOps::is_terminal_failure(
        AsyncRecoveryOwner::AuthRenewal
    ));
    assert_eq!(
        AsyncTimerRecoveryOps::retry_streak(AsyncRecoveryOwner::AuthRenewal),
        0
    );
    assert_eq!(
        AsyncTimerRecoveryOps::recovery_due(AsyncRecoveryOwner::AuthRenewal, u64::MAX),
        None
    );
}

#[test]
fn recovery_completion_arbitrates_pending_ensure_and_reconcile_requests() {
    reset();
    let owner = AsyncRecoveryOwner::AuthRenewal;
    AsyncTimerRecoveryOps::activate_recovery(owner, 10);
    let ensured = acquired(AsyncTimerRecoveryOps::claim(owner, 10, 20).expect("claim ensure pass"));
    assert!(AsyncTimerRecoveryOps::ensure_recovery(owner, 50));
    assert!(AsyncTimerRecoveryOps::ensure_recovery(owner, 40));
    assert!(
        AsyncTimerRecoveryOps::finish(ensured, AsyncRecoveryCompletion::Success, Some(30))
            .expect("finish ensured pass")
    );
    assert_eq!(AsyncTimerRecoveryOps::recovery_due(owner, 30), Some(30));

    let reconciled =
        acquired(AsyncTimerRecoveryOps::claim(owner, 30, 60).expect("claim reconcile pass"));
    assert!(AsyncTimerRecoveryOps::reconcile_recovery(owner, Some(80)));
    assert!(
        AsyncTimerRecoveryOps::finish(reconciled, AsyncRecoveryCompletion::Success, Some(20))
            .expect("finish reconciled pass")
    );
    assert_eq!(AsyncTimerRecoveryOps::recovery_due(owner, 79), None);
    assert_eq!(AsyncTimerRecoveryOps::recovery_due(owner, 80), Some(80));

    let cancelled =
        acquired(AsyncTimerRecoveryOps::claim(owner, 80, 90).expect("claim cancellation pass"));
    assert!(AsyncTimerRecoveryOps::reconcile_recovery(owner, None));
    assert!(
        AsyncTimerRecoveryOps::finish(cancelled, AsyncRecoveryCompletion::Success, Some(100))
            .expect("finish cancelled pass")
    );
    assert_eq!(AsyncTimerRecoveryOps::recovery_due(owner, u64::MAX), None);
}

#[test]
fn takeover_preserves_provider_pending_schedule_but_normal_completion_discards_its_mirror() {
    reset();
    let owner = AsyncRecoveryOwner::CycleTopup;
    let normal = acquired(AsyncTimerRecoveryOps::claim(owner, 1, 2).expect("claim normal pass"));
    AsyncTimerRecoveryOps::record_active_ensure(owner, 50);
    AsyncTimerRecoveryOps::record_active_ensure(owner, 40);
    AsyncTimerRecoveryOps::activate_recovery(owner, 2);
    let takeover = acquired(AsyncTimerRecoveryOps::claim(owner, 2, 10).expect("claim takeover"));
    assert!(
        !AsyncTimerRecoveryOps::finish(normal, AsyncRecoveryCompletion::Success, Some(60))
            .expect("reject stale normal completion")
    );
    assert!(
        AsyncTimerRecoveryOps::finish(takeover, AsyncRecoveryCompletion::Success, Some(60))
            .expect("finish takeover")
    );
    assert_eq!(AsyncTimerRecoveryOps::recovery_due(owner, 40), Some(40));

    reset();
    let normal = acquired(AsyncTimerRecoveryOps::claim(owner, 1, 2).expect("claim normal pass"));
    AsyncTimerRecoveryOps::record_active_reconcile(owner, Some(70));
    assert!(
        AsyncTimerRecoveryOps::finish(normal, AsyncRecoveryCompletion::Success, None)
            .expect("finish normal pass")
    );
    AsyncTimerRecoveryOps::activate_recovery(owner, 5);
    let recovery = acquired(AsyncTimerRecoveryOps::claim(owner, 5, 10).expect("claim recovery"));
    assert!(
        AsyncTimerRecoveryOps::finish(recovery, AsyncRecoveryCompletion::Success, Some(90))
            .expect("finish recovery")
    );
    assert_eq!(AsyncTimerRecoveryOps::recovery_due(owner, 89), None);
    assert_eq!(AsyncTimerRecoveryOps::recovery_due(owner, 90), Some(90));
}

#[test]
fn generation_overflow_is_fail_closed_without_mutation() {
    reset();
    let mut record = AsyncTimerRecoveryStore::export().record;
    record.cycle_topup = AsyncRecoveryOwnerRecord {
        last_attempt_generation: u64::MAX,
        ..AsyncRecoveryOwnerRecord::default()
    };
    AsyncTimerRecoveryStore::import(AsyncTimerRecoveryData { record });

    assert!(AsyncTimerRecoveryOps::claim(AsyncRecoveryOwner::CycleTopup, 1, 2).is_err());
    assert_eq!(
        AsyncTimerRecoveryStore::export()
            .record
            .cycle_topup
            .last_attempt_generation,
        u64::MAX
    );
}
