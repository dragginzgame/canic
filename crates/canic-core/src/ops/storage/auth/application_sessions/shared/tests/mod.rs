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
fn stored_projection_cannot_increase_shared_session_scopes() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store
        .transaction(Some(&login), |tx| tx.stage(session.clone(), replay))
        .unwrap();
    let mut state = LocalApplicationAuthorizationState::export();
    state.sessions[0].scopes.push("write".to_owned());
    LocalApplicationAuthorizationState::import(state);
    LocalApplicationAuthorizationStateOps::restore_application_session_state().unwrap();
    assert_eq!(
        store.transaction(None, |tx| tx.session(session.caller())),
        Err(SessionError::InvalidPolicyOrRecord)
    );
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
