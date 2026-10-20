//! Durable admission/logout transactions using current, cryptographically issued sessions.
//! Native storage proofs; installed lifecycle qualification remains separate.

mod fixture;

use super::*;
use crate::{
    model::auth::application_authorization::{
        LocalApplicationAuthoritySnapshot, MAX_APPLICATION_UNKNOWN_LOGOUTS,
    },
    test::{seams, support::fleet_key},
};
use fixture::{NOW, browser, pair};
use ic_auth::{admission_budget::SessionAdmissionBudget, portable::MemoryLoginRevocations};
use ic_auth_protocol_types::{LogoutResult, LogoutUnavailable};

fn store() -> ApplicationSessionStore {
    ApplicationSessionStore::new(&LocalApplicationAuthoritySnapshot::new(
        fleet_key(1),
        canic_contracts::ids::CanisterRole::new("app_service"),
        0,
    ))
}

fn restore_ledger() -> MemoryLoginRevocations {
    let data = LocalApplicationAuthorizationState::export();
    MemoryLoginRevocations::new(fixture::audience(), 4096, 256, 1_000_000_000, NOW)
        .unwrap()
        .restore_records(
            NOW,
            (data.login_totals.admitted, data.login_totals.unknown),
            data.logins
                .iter()
                .map(|(_, record)| LoginRevocationChange::decode(&record.bytes).unwrap()),
        )
        .unwrap()
}

#[test]
fn committed_pair_survives_restore_and_replacement_retains_consumption() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (first, replay, login) = pair(false);
    store
        .transaction(Some(&login), |tx| tx.stage(first.clone(), replay))
        .unwrap();
    let (second, replay, login) = pair(true);
    store
        .transaction(Some(&login), |tx| tx.stage(second.clone(), replay))
        .unwrap();
    let before_bytes = with_indexes(|i| i.encoded_bytes).unwrap();
    invalidate_indexes();
    let restored =
        LocalApplicationAuthorizationStateOps::restore_application_session_state().unwrap();
    assert_eq!((restored.sessions, restored.replays), (1, 2));
    assert_eq!(restored.stable_bytes, before_bytes);
    assert_eq!(restore_ledger().occupancy(), (1, 0));
    store
        .transaction(None, |tx| {
            assert_eq!(tx.session(second.caller())?, Some(second.clone()));
            assert!(tx.replay_exists(first.fingerprint())?);
            assert!(tx.replay_exists(second.fingerprint())?);
            Ok(())
        })
        .unwrap();
    let before = LocalApplicationAuthorizationState::export();
    let (session, replay, login) = pair(false);
    assert_eq!(
        store.transaction(Some(&login), |tx| tx.stage(session, replay)),
        Err(SessionError::ReplayConflict)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
}

#[test]
fn failure_after_staging_preserves_records_and_indexes() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let before = LocalApplicationAuthorizationState::export();
    let before_bytes = with_indexes(|i| i.encoded_bytes).unwrap();
    let (session, replay, login) = pair(false);
    let result: Result<(), SessionError> = store.transaction(Some(&login), |tx| {
        tx.stage(session, replay)?;
        Err(SessionError::StorageFailure)
    });
    assert_eq!(result, Err(SessionError::StorageFailure));
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    assert_eq!(with_indexes(|i| i.encoded_bytes).unwrap(), before_bytes);
    let (session, replay, _) = pair(false);
    assert_eq!(
        store.transaction(None, |tx| tx.stage(session, replay)),
        Err(SessionError::InvalidPolicyOrRecord)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
}

#[test]
fn corrupt_canonical_session_is_refused_without_installing_indexes() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store
        .transaction(Some(&login), |tx| tx.stage(session, replay))
        .unwrap();
    let mut corrupt = LocalApplicationAuthorizationState::export();
    corrupt.sessions[0].shared_session.push(0);
    LocalApplicationAuthorizationState::import(corrupt.clone());
    assert!(LocalApplicationAuthorizationStateOps::restore_application_session_state().is_err());
    assert_eq!(LocalApplicationAuthorizationState::export(), corrupt);
    assert!(with_indexes(|_| ()).is_err());
}

#[test]
fn stale_generation_and_exhausted_byte_budget_commit_neither_record() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let mut state = LocalApplicationAuthorizationState::export();
    state.authority_generation = 1;
    LocalApplicationAuthorizationState::import(state);
    LocalApplicationAuthorizationStateOps::restore_application_session_state().unwrap();
    let before = LocalApplicationAuthorizationState::export();
    let (session, replay, login) = pair(false);
    assert_eq!(
        store.transaction(Some(&login), |tx| tx.stage(session, replay)),
        Err(SessionError::InvalidPolicyOrRecord)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    let mut state = before;
    state.authority_generation = 0;
    LocalApplicationAuthorizationState::import(state);
    LocalApplicationAuthorizationStateOps::restore_application_session_state().unwrap();
    APPLICATION_SESSION_INDEXES.with_borrow_mut(|slot| {
        slot.as_mut().unwrap().encoded_bytes = MAX_APPLICATION_SESSION_STABLE_BYTES;
    });
    let before = LocalApplicationAuthorizationState::export();
    let (session, replay, login) = pair(false);
    assert_eq!(
        store.transaction(Some(&login), |tx| tx.stage(session, replay)),
        Err(SessionError::ByteCapacity)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
}

#[test]
fn logout_persists_denial_and_preserves_replay_through_restoration() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store
        .transaction(Some(&login), |tx| tx.stage(session.clone(), replay))
        .unwrap();
    let (other, replay, other_login) = fixture::pair_for_browser(false, 2);
    store
        .transaction(Some(&other_login), |tx| tx.stage(other.clone(), replay))
        .unwrap();
    let mut ledger = restore_ledger();
    let mut budget = SessionAdmissionBudget::new(32, 1, 0).unwrap();
    assert!(matches!(
        ic_auth::session::logout_session(
            &mut store,
            &mut ledger,
            &mut budget,
            session.caller(),
            NOW
        ),
        LogoutResult::Revoked { .. }
    ));
    let expected_bytes = with_indexes(|i| i.encoded_bytes).unwrap();
    invalidate_indexes();
    let stats = LocalApplicationAuthorizationStateOps::restore_application_session_state().unwrap();
    assert_eq!((stats.sessions, stats.replays), (1, 2));
    assert_eq!(
        store
            .transaction(None, |tx| tx.session(other.caller()))
            .unwrap(),
        Some(other)
    );
    assert_eq!(stats.stable_bytes, expected_bytes);
    assert!(restore_ledger().is_revoked(session.caller(), NOW).unwrap());
    let before = LocalApplicationAuthorizationState::export();
    assert_eq!(
        store.transaction(Some(&login), |tx| tx.remove(session.caller())),
        Err(SessionError::InvalidPolicyOrRecord)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
}

#[test]
fn unknown_logout_pool_is_bounded_without_erasing_admitted_logins() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store
        .transaction(Some(&login), |tx| tx.stage(session.clone(), replay))
        .unwrap();
    let mut ledger = restore_ledger();
    let mut budget = SessionAdmissionBudget::new(512, 1, 0).unwrap();
    for id in 0..MAX_APPLICATION_UNKNOWN_LOGOUTS {
        assert!(matches!(
            ic_auth::session::logout_session(
                &mut store,
                &mut ledger,
                &mut budget,
                browser(u16::try_from(id + 2).unwrap()),
                NOW
            ),
            LogoutResult::Revoked { .. }
        ));
    }
    let before = LocalApplicationAuthorizationState::export();
    assert_eq!(
        ic_auth::session::logout_session(&mut store, &mut ledger, &mut budget, browser(258), NOW),
        LogoutResult::Unavailable(LogoutUnavailable::Capacity)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    assert_eq!(
        restore_ledger().occupancy(),
        (1, MAX_APPLICATION_UNKNOWN_LOGOUTS)
    );
    assert_eq!(
        store
            .transaction(None, |tx| tx.session(session.caller()))
            .unwrap(),
        Some(session)
    );
    let mut missing = LocalApplicationAuthorizationState::export();
    missing.logins.clear();
    assert!(matches!(
        build_indexes(&missing),
        Err(ApplicationSessionStateError::InvalidLoginRecord)
    ));
}

#[test]
fn repeated_logout_cannot_replace_retained_account_binding() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store
        .transaction(Some(&login), |tx| tx.stage(session.clone(), replay))
        .unwrap();
    let mut ledger = restore_ledger();
    let mut budget = SessionAdmissionBudget::new(32, 1, 0).unwrap();
    let revoked = ic_auth::session::logout_session(
        &mut store,
        &mut ledger,
        &mut budget,
        session.caller(),
        NOW,
    );
    assert!(matches!(revoked, LogoutResult::Revoked { .. }));
    let before = LocalApplicationAuthorizationState::export();
    let before_bytes = with_indexes(|i| i.encoded_bytes).unwrap();

    // A second independently issued login uses the same transport key but another account.
    let (_, _, other_login) = fixture::pair_for_account(false, 1, Principal::from_slice(&[44]));
    let mut other_ledger =
        MemoryLoginRevocations::new(fixture::audience(), 4096, 256, 1_000_000_000, NOW)
            .unwrap()
            .restore_records(NOW, (1, 0), [other_login])
            .unwrap();
    assert_eq!(
        ic_auth::session::logout_session(
            &mut store,
            &mut other_ledger,
            &mut budget,
            session.caller(),
            NOW,
        ),
        LogoutResult::Unavailable(LogoutUnavailable::Storage),
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    assert_eq!(with_indexes(|i| i.encoded_bytes).unwrap(), before_bytes);
    assert!(!other_ledger.is_revoked(session.caller(), NOW).unwrap());

    // The matching retry remains idempotent and preserves replay custody.
    assert_eq!(
        ic_auth::session::logout_session(
            &mut store,
            &mut ledger,
            &mut budget,
            session.caller(),
            NOW,
        ),
        revoked
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    invalidate_indexes();
    let restored =
        LocalApplicationAuthorizationStateOps::restore_application_session_state().unwrap();
    assert_eq!((restored.sessions, restored.replays), (0, 1));
    assert!(restore_ledger().is_revoked(session.caller(), NOW).unwrap());
}

#[test]
fn restoration_rejects_login_binding_mismatch_and_revoked_live_session() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store
        .transaction(Some(&login), |tx| tx.stage(session.clone(), replay))
        .unwrap();
    let original = LocalApplicationAuthorizationState::export();
    let (_, _, other_login) = fixture::pair_for_account(false, 1, Principal::from_slice(&[44]));
    let mut mismatched = original.clone();
    mismatched.logins[0].1.bytes = other_login.encode().unwrap();
    assert!(matches!(
        build_indexes(&mismatched),
        Err(ApplicationSessionStateError::InvalidLoginRecord)
    ));

    let mut ledger = restore_ledger();
    let mut budget = SessionAdmissionBudget::new(32, 1, 0).unwrap();
    let mut revoked = original.clone();
    ledger
        .with_revocation(&mut budget, session.caller(), NOW, |change| {
            revoked.logins[0].1.bytes = change.encode()?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        build_indexes(&revoked),
        Err(ApplicationSessionStateError::InvalidLoginRecord)
    ));
    // Denial without a session is a complete logout; its replay is still retained.
    revoked.sessions.clear();
    assert!(build_indexes(&revoked).is_ok());
    assert!(build_indexes(&original).is_ok());
    assert_eq!(LocalApplicationAuthorizationState::export(), original);
}

#[test]
fn shared_authorization_enforces_expiry_scope_generation_and_revocation() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store
        .transaction(Some(&login), |tx| tx.stage(session.clone(), replay))
        .unwrap();
    let mut ledger = restore_ledger();
    assert_eq!(
        fixture::authorize(&mut store, &mut ledger, NOW, 0, &["read".into()]),
        Ok(session.clone())
    );
    assert_eq!(
        fixture::authorize(&mut store, &mut ledger, NOW, 0, &["write".into()]),
        Err(SessionError::ScopeRejected)
    );
    assert_eq!(
        fixture::authorize(&mut store, &mut ledger, NOW, 1, &[]),
        Err(SessionError::StaleAuthority)
    );
    let mut budget = SessionAdmissionBudget::new(32, 1, 0).unwrap();
    assert!(matches!(
        ic_auth::session::logout_session(
            &mut store,
            &mut ledger,
            &mut budget,
            session.caller(),
            NOW
        ),
        LogoutResult::Revoked { .. }
    ));
    assert!(matches!(
        fixture::authorize(&mut store, &mut ledger, NOW, 0, &[]),
        Err(SessionError::Revocation(_))
    ));
}

#[test]
fn canonical_sessions_restore_and_cleanup_at_the_exclusive_deadline() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store
        .transaction(Some(&login), |tx| tx.stage(session.clone(), replay))
        .unwrap();
    let mut ledger = restore_ledger();
    assert_eq!(
        fixture::authorize(&mut store, &mut ledger, session.expires_at_ns(), 0, &[]),
        Err(SessionError::MissingOrExpired)
    );
    assert_eq!(
        LocalApplicationAuthorizationStateOps::application_session_cleanup_due_at_ns().unwrap(),
        Some(NOW + 2_000_000_000)
    );
    assert_eq!(
        LocalApplicationAuthorizationStateOps::cleanup_application_sessions(
            session.expires_at_ns()
        )
        .unwrap()
        .sessions_removed,
        1
    );
    assert_eq!(
        LocalApplicationAuthorizationState::export().logins.len(),
        1,
        "session expiry cannot erase the login reservation"
    );
}

#[test]
fn cleanup_preserves_replay_limits_and_rejects_corruption_before_writes() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut data = LocalApplicationAuthorizationStateData::default();
    for id in 0..129_u16 {
        let mut fingerprint = [0; 32];
        fingerprint[..2].copy_from_slice(&id.to_be_bytes());
        data.replays.push(LocalApplicationReplayRecord {
            proof_fingerprint: fingerprint,
            transport_caller: browser(id),
            authority_generation: 0,
            remove_at_ns: NOW,
        });
    }
    LocalApplicationAuthorizationState::import(data);
    LocalApplicationAuthorizationStateOps::restore_application_session_state().unwrap();
    assert_eq!(
        LocalApplicationAuthorizationStateOps::cleanup_application_sessions(NOW)
            .unwrap()
            .replays_removed,
        128
    );
    assert_eq!(
        LocalApplicationAuthorizationState::export().replays.len(),
        1
    );
    let mut data = LocalApplicationAuthorizationState::export();
    data.sessions.push(LocalApplicationSessionRecord {
        shared_session: vec![0],
    });
    LocalApplicationAuthorizationState::import(data.clone());
    assert!(LocalApplicationAuthorizationStateOps::cleanup_application_sessions(NOW).is_err());
    assert_eq!(LocalApplicationAuthorizationState::export(), data);
    assert!(
        LocalApplicationAuthorizationStateOps::application_session_cleanup_due_at_ns().is_err()
    );
}

#[test]
fn restoration_requires_unique_sessions_and_a_reservation_for_every_session() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store
        .transaction(Some(&login), |tx| tx.stage(session, replay))
        .unwrap();
    let original = LocalApplicationAuthorizationState::export();
    let mut duplicated = original.clone();
    duplicated.sessions.push(duplicated.sessions[0].clone());
    assert!(matches!(
        build_indexes(&duplicated),
        Err(ApplicationSessionStateError::DuplicateCaller)
    ));
    let mut missing = original;
    missing.logins.clear();
    missing.login_totals =
        crate::storage::stable::auth::LocalApplicationLoginTotalsRecord::default();
    assert!(matches!(
        build_indexes(&missing),
        Err(ApplicationSessionStateError::InvalidLoginRecord)
    ));
}

#[test]
fn per_subject_capacity_is_refused_before_persistence() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    for device in 1..=MAX_ACTIVE_APPLICATION_SESSIONS_PER_SUBJECT {
        let (session, replay, login) =
            fixture::pair_for_browser(false, u16::try_from(device).unwrap());
        store
            .transaction(Some(&login), |tx| tx.stage(session, replay))
            .unwrap();
    }
    let before = LocalApplicationAuthorizationState::export();
    let (session, replay, login) = fixture::pair_for_browser(false, 129);
    assert_eq!(
        store.transaction(Some(&login), |tx| tx.stage(session, replay)),
        Err(SessionError::Capacity)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    invalidate_indexes();
    assert_eq!(
        LocalApplicationAuthorizationStateOps::restore_application_session_state()
            .unwrap()
            .sessions,
        MAX_ACTIVE_APPLICATION_SESSIONS_PER_SUBJECT
    );
}

#[test]
fn maximum_admitted_state_reconstructs_within_stable_and_heap_bounds() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    for id in 1..=MAX_ACTIVE_APPLICATION_SESSIONS {
        let device = u16::try_from(id).unwrap();
        for second in [false, true] {
            let (session, replay, login) =
                fixture::pair_for_account(second, device, browser(device));
            store
                .transaction(Some(&login), |tx| tx.stage(session, replay))
                .unwrap();
        }
    }
    invalidate_indexes();
    let stats = LocalApplicationAuthorizationStateOps::restore_application_session_state().unwrap();
    assert_eq!(stats.sessions, MAX_ACTIVE_APPLICATION_SESSIONS);
    assert_eq!(stats.replays, MAX_APPLICATION_REPLAY_RECORDS);
    assert!(stats.stable_bytes <= MAX_APPLICATION_SESSION_STABLE_BYTES);
    assert!(stats.estimated_index_bytes <= MAX_APPLICATION_SESSION_INDEX_BYTES);
}

#[test]
fn replay_capacity_is_owned_by_each_transport_caller() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut state = LocalApplicationAuthorizationStateData::default();
    for id in 0..=MAX_APPLICATION_REPLAY_RECORDS_PER_CALLER {
        let mut record = LocalApplicationReplayRecord {
            proof_fingerprint: [0; 32],
            transport_caller: browser(1),
            authority_generation: 0,
            remove_at_ns: NOW + 1,
        };
        record.proof_fingerprint[..8].copy_from_slice(&(id as u64).to_be_bytes());
        state.replays.push(record);
    }
    assert!(matches!(
        build_indexes(&state),
        Err(ApplicationSessionStateError::ReplayCallerCapacity)
    ));
    state.replays.last_mut().unwrap().transport_caller = browser(2);
    let (indexes, _) = build_indexes(&state).unwrap();
    assert_eq!(
        indexes.replay_count_by_caller.get(&browser(1)),
        Some(&MAX_APPLICATION_REPLAY_RECORDS_PER_CALLER)
    );
    assert_eq!(indexes.replay_count_by_caller.get(&browser(2)), Some(&1));
}

#[test]
fn restore_rejects_global_over_capacity_before_admitting_records() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let (session, replay, _) = pair(false);
    let mut data = LocalApplicationAuthorizationStateData {
        sessions: vec![session_to_record(&session).unwrap(); MAX_ACTIVE_APPLICATION_SESSIONS + 1],
        ..LocalApplicationAuthorizationStateData::default()
    };
    assert!(matches!(
        build_indexes(&data),
        Err(ApplicationSessionStateError::ActiveGlobalCapacity)
    ));
    data.sessions.clear();
    data.replays = vec![
        LocalApplicationReplayRecord {
            proof_fingerprint: replay.fingerprint(),
            transport_caller: replay.caller(),
            authority_generation: replay.generation(),
            remove_at_ns: replay.remove_at_ns()
        };
        MAX_APPLICATION_REPLAY_RECORDS + 1
    ];
    assert!(matches!(
        build_indexes(&data),
        Err(ApplicationSessionStateError::ReplayGlobalCapacity)
    ));
}
