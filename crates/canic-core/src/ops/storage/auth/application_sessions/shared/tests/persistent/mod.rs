//! Retained persistent receiver state through the actual Auth engine.
//! Native stable-store proofs; installed upgrade qualification remains separate.

use super::*;

fn persistent_policy() -> PersistentReceiverPolicy {
    PersistentReceiverPolicy {
        fleet: fleet_key(1),
        clock_skew_ns: 1_000_000_000,
        verification_burst: 1,
        verification_refill_interval_ns: 100,
        unknown_logout_burst: 1,
        unknown_logout_refill_interval_ns: 100,
    }
}

#[test]
fn persistent_receiver_retains_independent_budgets_and_original_lease_after_restore() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let now = NOW + 100;
    let mut receiver = PersistentReceiverState::restore(persistent_policy(), now - 100).unwrap();
    let credential = fixture::issue(false, 1, Principal::from_slice(&[43]), 42);
    let attempt = |receiver: &mut PersistentReceiverState, now| {
        receiver.run(now, |store, ledger, verification, _| {
            fixture::establish_issued(store, ledger, verification, &credential, browser(1), now)
        })
    };
    assert_eq!(
        attempt(&mut receiver, now - 100),
        Err(exhausted_signature_budget())
    );
    let unknown_caller = browser(3).into();
    let logout = receiver
        .run(now, |store, ledger, _, unknown| {
            Ok(ic_auth::session::logout_session(
                store,
                ledger,
                unknown,
                unknown_caller,
                now,
            ))
        })
        .unwrap();
    assert!(matches!(logout, LogoutResult::Revoked { .. }));
    let ic_auth::session::SessionEstablished::Created(session) =
        attempt(&mut receiver, now).unwrap()
    else {
        panic!("fresh credential must create its session")
    };
    // An exact retry has no need for new verification credit.
    assert_eq!(
        attempt(&mut receiver, now),
        Ok(ic_auth::session::SessionEstablished::ExactRetry(
            session.clone()
        ))
    );
    let other = fixture::issue(false, 2, Principal::from_slice(&[43]), 42);
    assert_eq!(
        receiver.run(now, |store, ledger, verification, _| {
            fixture::establish_issued(store, ledger, verification, &other, browser(2), now)
        }),
        Err(exhausted_signature_budget())
    );
    assert_eq!(
        receiver.run(now, |store, ledger, _, unknown| {
            Ok(ic_auth::session::logout_session(
                store,
                ledger,
                unknown,
                browser(4).into(),
                now,
            ))
        }),
        Ok(LogoutResult::Unavailable(LogoutUnavailable::Capacity))
    );
    let checkpoint = LocalApplicationAuthorizationState::export();
    let mut restored = PersistentReceiverState::restore(persistent_policy(), now + 1).unwrap();
    assert_eq!(
        receiver.run(now + 1, |_, _, _, _| Ok(())),
        Err(SessionError::StorageFailure)
    );
    for expired_enrollment in [true, false] {
        let result = restored.run(now + 1, |store, ledger, _, _| {
            assert_eq!(ledger.occupancy(), (1, 1));
            fixture::authorize(store, ledger, now + 1, 0, &["read".into()], |authority| {
                if expired_enrollment {
                    authority.enrollment_expires_at_ns = now;
                }
            })
        });
        if expired_enrollment {
            assert_eq!(result, Err(SessionError::StaleAuthority));
        } else {
            assert_eq!(result, Ok(session.clone()));
        }
    }
    let after = LocalApplicationAuthorizationState::export();
    assert_eq!(after.sessions, checkpoint.sessions);
    assert_eq!(after.replays, checkpoint.replays);
    assert_eq!(after.logins, checkpoint.logins);
    assert_eq!(after.observed_at_ns, now + 1);
    assert_eq!(
        attempt(&mut restored, now + 1),
        Ok(ic_auth::session::SessionEstablished::ExactRetry(session))
    );
}

#[test]
fn persistent_receiver_refused_commit_retains_credit_charge_and_clock() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut receiver = PersistentReceiverState::restore(persistent_policy(), NOW - 100).unwrap();
    APPLICATION_SESSION_INDEXES.with_borrow_mut(|slot| {
        slot.as_mut().unwrap().encoded_bytes = MAX_APPLICATION_SESSION_STABLE_BYTES;
    });
    let credential = fixture::issue(false, 1, Principal::from_slice(&[43]), 42);
    for expected in [SessionError::ByteCapacity, exhausted_signature_budget()] {
        assert_eq!(
            receiver.run(NOW, |store, ledger, verification, _| {
                fixture::establish_issued(store, ledger, verification, &credential, browser(1), NOW)
            }),
            Err(expected)
        );
        let state = LocalApplicationAuthorizationState::export();
        assert!(state.sessions.is_empty() && state.replays.is_empty() && state.logins.is_empty());
        assert_eq!(state.observed_at_ns, NOW);
    }
    assert_eq!(
        receiver.run(NOW - 1, |_, _, _, _| Ok(())),
        Err(SessionError::AdmissionClockRollback)
    );
    assert_eq!(
        receiver.run(NOW, |_, _, _, _| Ok(())),
        Err(SessionError::StorageFailure)
    );
    assert_eq!(
        LocalApplicationAuthorizationState::export().observed_at_ns,
        NOW
    );
}

#[test]
fn persistent_receiver_restore_refusals_close_previous_owner_and_preserve_records() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut receiver = PersistentReceiverState::restore(persistent_policy(), NOW - 100).unwrap();
    receiver
        .run(NOW, |store, ledger, verification, _| {
            fixture::establish(
                store,
                ledger,
                verification,
                false,
                1,
                Principal::from_slice(&[43]),
            )
        })
        .unwrap();
    let checkpoint = LocalApplicationAuthorizationState::export();
    assert!(matches!(
        PersistentReceiverState::restore(persistent_policy(), NOW - 1),
        Err(SessionError::AdmissionClockRollback)
    ));
    assert!(with_indexes(|_| ()).is_err());
    assert_eq!(LocalApplicationAuthorizationState::export(), checkpoint);
    assert_eq!(
        receiver.run(NOW, |_, _, _, _| Ok(())),
        Err(SessionError::StorageFailure)
    );
    let mut wrong_fleet = persistent_policy();
    wrong_fleet.fleet = fleet_key(2);
    assert!(PersistentReceiverState::restore(wrong_fleet, NOW).is_err());
    assert!(with_indexes(|_| ()).is_err());
    let mut wrong_skew = persistent_policy();
    wrong_skew.clock_skew_ns += 1;
    assert!(PersistentReceiverState::restore(wrong_skew, NOW).is_err());
    assert!(with_indexes(|_| ()).is_err());
    let mut invalid_budget = persistent_policy();
    invalid_budget.verification_burst = 0;
    assert!(matches!(
        PersistentReceiverState::restore(invalid_budget, NOW),
        Err(SessionError::InvalidPolicyOrRecord)
    ));
    assert_eq!(LocalApplicationAuthorizationState::export(), checkpoint);
    assert!(PersistentReceiverState::restore(persistent_policy(), NOW).is_ok());
}

#[test]
fn persistent_receiver_retains_committed_logout_and_bounded_pruning() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut receiver = PersistentReceiverState::restore(persistent_policy(), NOW - 100).unwrap();
    receiver
        .run(NOW, |store, ledger, verification, _| {
            fixture::establish(
                store,
                ledger,
                verification,
                false,
                1,
                Principal::from_slice(&[43]),
            )
        })
        .unwrap();
    let logout = receiver
        .run(NOW, |store, ledger, _, unknown| {
            Ok(ic_auth::session::logout_session(
                store,
                ledger,
                unknown,
                browser(1).into(),
                NOW,
            ))
        })
        .unwrap();
    let LogoutResult::Revoked { deny_until_ns } = logout else {
        panic!("committed logout")
    };
    let checkpoint = LocalApplicationAuthorizationState::export();
    let mut restored = PersistentReceiverState::restore(persistent_policy(), NOW + 1).unwrap();
    assert_eq!(
        restored.run(NOW + 1, |store, ledger, _, _| {
            fixture::authorize(store, ledger, NOW + 1, 0, &["read".into()], |_| {})
        }),
        Err(SessionError::Revocation(
            ic_auth::portable::LoginRevocationError::Revoked
        ))
    );
    assert_eq!(
        LocalApplicationAuthorizationState::export().logins,
        checkpoint.logins
    );
    assert_eq!(
        LocalApplicationAuthorizationState::export().replays,
        checkpoint.replays
    );
    assert_eq!(restored.prune_logins(deny_until_ns - 1, 1), Ok(0));
    assert_eq!(restored.prune_logins(deny_until_ns, 1), Ok(1));
    assert_eq!(
        restored.run(deny_until_ns, |_, ledger, _, _| Ok(ledger.occupancy())),
        Ok((0, 0))
    );
    assert!(
        LocalApplicationAuthorizationState::export()
            .logins
            .is_empty()
    );
    assert_eq!(
        LocalApplicationAuthorizationState::export().replays,
        checkpoint.replays
    );
}

#[test]
fn persistent_receiver_maintenance_keeps_its_ledger_and_fences_external_maintenance() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut receiver = PersistentReceiverState::restore(persistent_policy(), NOW - 100).unwrap();
    let established = receiver
        .run(NOW, |store, ledger, verification, _| {
            fixture::establish(
                store,
                ledger,
                verification,
                false,
                1,
                Principal::from_slice(&[43]),
            )
        })
        .unwrap();
    let ic_auth::session::SessionEstablished::Created(session) = established else {
        panic!("created")
    };
    let until = LocalApplicationAuthorizationState::export().logins[0]
        .1
        .clone();
    let until = LoginRevocationChange::decode(&until.bytes)
        .unwrap()
        .until_ns();
    assert_eq!(
        receiver
            .cleanup_sessions(session.expires_at_ns())
            .unwrap()
            .sessions_removed,
        1
    );
    assert_eq!(
        receiver.run(session.expires_at_ns(), |_, ledger, _, _| Ok(
            ledger.occupancy()
        )),
        Ok((1, 0))
    );
    receiver.cleanup_sessions(until).unwrap();
    assert_eq!(receiver.prune_logins(until, 1), Ok(1));
    assert_eq!(
        receiver.run(until, |_, ledger, _, _| Ok(ledger.occupancy())),
        Ok((0, 0))
    );
    LocalApplicationAuthorizationStateOps::cleanup_application_sessions(until).unwrap();
    assert_eq!(
        receiver.run(until, |_, _, _, _| Ok(())),
        Err(SessionError::StorageFailure)
    );
}

#[test]
fn persistent_receiver_failed_pruning_fences_the_owner_and_restores_exact_rows() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut receiver = PersistentReceiverState::restore(persistent_policy(), NOW - 100).unwrap();
    receiver
        .run(NOW, |store, ledger, verification, _| {
            fixture::establish(
                store,
                ledger,
                verification,
                false,
                1,
                Principal::from_slice(&[43]),
            )
        })
        .unwrap();
    let before = LocalApplicationAuthorizationState::export();
    let until = LoginRevocationChange::decode(&before.logins[0].1.bytes)
        .unwrap()
        .until_ns();
    assert!(receiver.prune_logins(until, 1).is_err());
    assert_eq!(
        receiver.run(until, |_, _, _, _| Ok(())),
        Err(SessionError::StorageFailure)
    );
    let after = LocalApplicationAuthorizationState::export();
    assert_eq!(after.sessions, before.sessions);
    assert_eq!(after.replays, before.replays);
    assert_eq!(after.logins, before.logins);
    assert_eq!(after.observed_at_ns, until);
    let mut restored = PersistentReceiverState::restore(persistent_policy(), until).unwrap();
    restored.cleanup_sessions(until).unwrap();
    assert_eq!(restored.prune_logins(until, 1), Ok(1));
}
