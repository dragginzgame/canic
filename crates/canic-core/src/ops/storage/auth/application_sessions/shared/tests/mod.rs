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
use ic_auth_protocol_types::{AdmissionRefusal, LogoutResult, LogoutUnavailable};

fn store() -> ApplicationSessionStore {
    ApplicationSessionStore::new(&LocalApplicationAuthoritySnapshot::new(
        fleet_key(1),
        canic_contracts::ids::CanisterRole::new("app_service"),
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
    store.admit(first.clone(), replay, &login).unwrap();
    let (second, replay, login) = pair(true);
    store.admit(second.clone(), replay, &login).unwrap();
    let before_bytes = with_indexes(|i| i.encoded_bytes).unwrap();
    invalidate_indexes();
    let restored = LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
    assert_eq!((restored.sessions, restored.replays), (1, 2));
    assert_eq!(restored.stable_bytes, before_bytes);
    assert_eq!(restore_ledger().occupancy(), (1, 0));
    store
        .read(|tx| {
            let session = tx.session(second.caller())?;
            assert!(matches!(&session, Some(Cow::Owned(_))));
            assert_eq!(session.as_deref(), Some(&second));
            assert!(tx.replay_exists(first.fingerprint())?);
            assert!(tx.replay_exists(second.fingerprint())?);
            Ok(())
        })
        .unwrap();
    let before = LocalApplicationAuthorizationState::export();
    let (session, replay, login) = pair(false);
    assert_eq!(
        store.admit(session, replay, &login),
        Err(SessionError::ReplayConflict)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
}

#[test]
fn mismatched_admission_preserves_records_and_indexes() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let before = LocalApplicationAuthorizationState::export();
    let before_bytes = with_indexes(|i| i.encoded_bytes).unwrap();
    let (session, _, login) = pair(false);
    let (_, different_replay, _) = pair(true);
    assert_eq!(
        store.admit(session, different_replay, &login),
        Err(SessionError::InvalidPolicyOrRecord)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    assert_eq!(with_indexes(|i| i.encoded_bytes).unwrap(), before_bytes);
}

#[test]
fn corrupt_canonical_session_is_refused_without_installing_indexes() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store.admit(session, replay, &login).unwrap();
    let mut corrupt = LocalApplicationAuthorizationState::export();
    corrupt.sessions[0].shared_session.push(0);
    LocalApplicationAuthorizationState::import(corrupt.clone());
    assert!(
        LocalApplicationAuthorizationStateOps::restore_application_session_state(
            LocalApplicationAuthorizationState::export().observed_at_ns
        )
        .is_err()
    );
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
    LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
    let before = LocalApplicationAuthorizationState::export();
    let (session, replay, login) = pair(false);
    assert_eq!(
        store.admit(session, replay, &login),
        Err(SessionError::InvalidPolicyOrRecord)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    let mut state = before;
    state.authority_generation = 0;
    LocalApplicationAuthorizationState::import(state);
    LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
    APPLICATION_SESSION_INDEXES.with_borrow_mut(|slot| {
        slot.as_mut().unwrap().encoded_bytes = MAX_APPLICATION_SESSION_STABLE_BYTES;
    });
    let before = LocalApplicationAuthorizationState::export();
    let (session, replay, login) = pair(false);
    assert_eq!(
        store.admit(session, replay, &login),
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
    store.admit(session.clone(), replay, &login).unwrap();
    let (other, replay, other_login) = fixture::pair_for_browser(false, 2);
    store.admit(other.clone(), replay, &other_login).unwrap();
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
    let stats = LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
    assert_eq!((stats.sessions, stats.replays), (1, 2));
    store
        .read(|tx| {
            assert_eq!(tx.session(other.caller())?.as_deref(), Some(&other));
            Ok(())
        })
        .unwrap();
    assert_eq!(stats.stable_bytes, expected_bytes);
    assert!(restore_ledger().is_revoked(session.caller(), NOW).unwrap());
    let before = LocalApplicationAuthorizationState::export();
    assert_eq!(
        store.logout(&login),
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
    store.admit(session.clone(), replay, &login).unwrap();
    let mut ledger = restore_ledger();
    let mut budget = SessionAdmissionBudget::new(512, 1, 0).unwrap();
    for id in 0..MAX_APPLICATION_UNKNOWN_LOGOUTS {
        assert!(matches!(
            ic_auth::session::logout_session(
                &mut store,
                &mut ledger,
                &mut budget,
                browser(u16::try_from(id + 2).unwrap()).into(),
                NOW
            ),
            LogoutResult::Revoked { .. }
        ));
    }
    let before = LocalApplicationAuthorizationState::export();
    assert_eq!(
        ic_auth::session::logout_session(
            &mut store,
            &mut ledger,
            &mut budget,
            browser(258).into(),
            NOW
        ),
        LogoutResult::Unavailable(LogoutUnavailable::Capacity)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    assert_eq!(
        restore_ledger().occupancy(),
        (1, MAX_APPLICATION_UNKNOWN_LOGOUTS)
    );
    store
        .read(|tx| {
            assert_eq!(tx.session(session.caller())?.as_deref(), Some(&session));
            Ok(())
        })
        .unwrap();
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
    store.admit(session.clone(), replay, &login).unwrap();
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
    let restored = LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
    assert_eq!((restored.sessions, restored.replays), (0, 1));
    assert!(restore_ledger().is_revoked(session.caller(), NOW).unwrap());
}

#[test]
fn restoration_rejects_login_binding_mismatch_and_revoked_live_session() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store.admit(session.clone(), replay, &login).unwrap();
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
        .with_revocation::<ic_auth::portable::LoginRevocationError>(
            &mut budget,
            session.caller(),
            NOW,
            |change| {
                revoked.logins[0].1.bytes = change.encode()?;
                Ok(())
            },
        )
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
    store.admit(session.clone(), replay, &login).unwrap();
    let mut ledger = restore_ledger();
    assert_eq!(
        fixture::authorize(&mut store, &mut ledger, NOW, 0, &["read".into()], |_| {}),
        Ok(session.clone())
    );
    assert_eq!(
        fixture::authorize(&mut store, &mut ledger, NOW, 0, &["write".into()], |_| {}),
        Err(SessionError::ScopeRejected)
    );
    assert_eq!(
        fixture::authorize(&mut store, &mut ledger, NOW, 1, &[], |_| {}),
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
        fixture::authorize(&mut store, &mut ledger, NOW, 0, &[], |_| {}),
        Err(SessionError::Revocation(_))
    ));
}

#[test]
fn canonical_sessions_restore_and_cleanup_at_the_exclusive_deadline() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store.admit(session.clone(), replay, &login).unwrap();
    let mut ledger = restore_ledger();
    assert_eq!(
        fixture::authorize(
            &mut store,
            &mut ledger,
            session.expires_at_ns(),
            0,
            &[],
            |_| {}
        ),
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
    LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
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
    store.admit(session, replay, &login).unwrap();
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
fn maximum_admitted_state_reconstructs_within_stable_and_heap_bounds() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    for id in 1..=MAX_ACTIVE_APPLICATION_SESSIONS {
        let device = u16::try_from(id).unwrap();
        for second in [false, true] {
            let (session, replay, login) = fixture::pair_for_browser(second, device);
            store
                .admit(session.clone(), replay.clone(), &login)
                .unwrap();
        }
    }
    invalidate_indexes();
    let stats = LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
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
            transport_caller: replay.caller().into(),
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

#[test]
fn engine_admission_commits_directly_and_exact_retry_does_not_verify_again() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let mut logins = fixture::ledger();
    let mut budget = SessionAdmissionBudget::new(1, 1, NOW - 1).unwrap();
    let account = Principal::from_slice(&[43]);
    let created =
        fixture::establish(&mut store, &mut logins, &mut budget, false, 1, account).unwrap();
    let ic_auth::session::SessionEstablished::Created(session) = created else {
        panic!("fresh admission must create a session");
    };
    let before = LocalApplicationAuthorizationState::export();
    assert_eq!(
        fixture::establish(&mut store, &mut logins, &mut budget, false, 1, account),
        Ok(ic_auth::session::SessionEstablished::ExactRetry(session)),
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    assert_eq!(
        fixture::establish(&mut store, &mut logins, &mut budget, true, 1, account),
        Err(exhausted_signature_budget()),
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    invalidate_indexes();
    LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
    let mut restored = restore_ledger();
    assert_eq!(
        fixture::authorize(&mut store, &mut restored, NOW, 0, &["read".into()], |_| {})
            .map(|value| ic_auth::session::SessionReceipt::from(&value)),
        Ok(session)
    );
    let LogoutResult::Revoked { deny_until_ns } = ic_auth::session::logout_session(
        &mut store,
        &mut restored,
        &mut budget,
        browser(1).into(),
        NOW,
    ) else {
        panic!("admitted caller logout must revoke");
    };
    assert!(deny_until_ns >= NOW + 60_000_000_000);
    assert_eq!(
        restore_ledger().records().next().unwrap().until_ns(),
        deny_until_ns
    );
    assert!(matches!(
        fixture::authorize(&mut store, &mut restored, NOW, 0, &["read".into()], |_| {}),
        Err(SessionError::Revocation(
            ic_auth::portable::LoginRevocationError::Revoked
        )),
    ));
}

/// Verification errors retain the shared verifier's typed budget failure.
fn exhausted_signature_budget() -> SessionError {
    SessionError::Credential(
        ic_auth::portable_credential::PortableCredentialError::Ed25519(
            ic_auth::portable_signature::PortableSignatureError::Budget(
                ic_auth::admission_budget::AdmissionBudgetError::Exhausted,
            ),
        ),
    )
}

#[test]
fn engine_commit_refusal_preserves_storage_and_ledger_but_charges_verification() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut owner = store();
    owner.fleet = fleet_key(2);
    let mut logins = fixture::ledger();
    let mut budget = SessionAdmissionBudget::new(1, 1, NOW - 1).unwrap();
    let account = Principal::from_slice(&[43]);
    let before = LocalApplicationAuthorizationState::export();
    let before_bytes = with_indexes(|i| i.encoded_bytes).unwrap();

    // The real commit owner refuses the otherwise valid proof's Fleet binding.
    assert_eq!(
        fixture::establish(&mut owner, &mut logins, &mut budget, false, 1, account),
        Err(SessionError::InvalidPolicyOrRecord),
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    assert_eq!(with_indexes(|i| i.encoded_bytes).unwrap(), before_bytes);
    assert_eq!(logins.occupancy(), (0, 0));
    assert!(logins.records().next().is_none());

    owner.fleet = fleet_key(1);
    assert_eq!(
        fixture::establish(&mut owner, &mut logins, &mut budget, false, 1, account),
        Err(exhausted_signature_budget()),
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    assert_eq!(logins.occupancy(), (0, 0));

    // A separately credited budget proves the failed commit consumed no replay.
    let mut credited = SessionAdmissionBudget::new(1, 1, NOW - 1).unwrap();
    assert!(matches!(
        fixture::establish(&mut owner, &mut logins, &mut credited, false, 1, account),
        Ok(ic_auth::session::SessionEstablished::Created(_)),
    ));
    assert_eq!(logins.occupancy(), (1, 0));
    invalidate_indexes();
    let restored = LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
    assert_eq!((restored.sessions, restored.replays), (1, 1));
    assert_eq!(restore_ledger().occupancy(), (1, 0));
}

fn logout_record(
    audience: ic_auth_protocol_types::AudienceId,
    caller: Principal,
) -> crate::storage::stable::auth::LocalApplicationLoginRecord {
    let mut ledger = MemoryLoginRevocations::new(audience, 4096, 256, 1_000_000_000, 0).unwrap();
    let mut budget = SessionAdmissionBudget::new(1, 1, 0).unwrap();
    ledger
        .with_revocation::<ic_auth::portable::LoginRevocationError>(
            &mut budget,
            caller.into(),
            NOW,
            |_| Ok(()),
        )
        .unwrap();
    crate::storage::stable::auth::LocalApplicationLoginRecord {
        bytes: ledger.records().next().unwrap().encode().unwrap(),
    }
}

#[test]
fn restored_logout_records_are_bound_to_the_protected_fleet() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut state = LocalApplicationAuthorizationState::export();
    state
        .logins
        .push((browser(1), logout_record(fixture::audience(), browser(1))));
    state.login_totals.unknown = 1;
    let binding = LocalApplicationAuthorityBindingRecord::Enabled {
        fleet: fleet_key(2),
        role: canic_contracts::ids::CanisterRole::new("app_service"),
        verifier_root_canister_id: Principal::from_slice(&[42]),
        minimum_accepted_registry_epoch: None,
        allowed_scopes: vec!["read".into()],
        maximum_session_ttl_secs: 30,
    };
    state.authority_binding = Some(binding);
    LocalApplicationAuthorizationState::import(state.clone());
    let persisted = LocalApplicationAuthorizationState::export();
    assert_eq!(
        LocalApplicationAuthorizationStateOps::restore_application_session_state(
            LocalApplicationAuthorizationState::export().observed_at_ns
        ),
        Err(ApplicationSessionStateError::InvalidLoginRecord),
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), persisted);
    assert_eq!(
        with_indexes(|_| ()),
        Err(ApplicationSessionStateError::IndexesUnavailable)
    );

    let Some(LocalApplicationAuthorityBindingRecord::Enabled { fleet, .. }) =
        &mut state.authority_binding
    else {
        panic!("enabled fixture authority");
    };
    *fleet = fleet_key(1);
    LocalApplicationAuthorizationState::import(state);
    LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
    assert!(restore_ledger().is_revoked(browser(1).into(), NOW).unwrap());
}

#[test]
fn disabled_authority_retains_denials_but_rejects_mixed_audiences() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut state = LocalApplicationAuthorizationState::export();
    state.authority_binding = Some(LocalApplicationAuthorityBindingRecord::Disabled);
    state
        .logins
        .push((browser(1), logout_record(fixture::audience(), browser(1))));
    state.login_totals.unknown = 1;
    LocalApplicationAuthorizationState::import(state.clone());
    LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
    assert!(restore_ledger().is_revoked(browser(1).into(), NOW).unwrap());

    let mut foreign = fixture::audience();
    foreign.fleet_id =
        ic_auth_protocol_types::CanonicalId::from_bytes(*fleet_key(2).fleet_id.as_bytes());
    state
        .logins
        .push((browser(2), logout_record(foreign, browser(2))));
    state.login_totals.unknown = 2;
    LocalApplicationAuthorizationState::import(state.clone());
    let persisted = LocalApplicationAuthorizationState::export();
    assert_eq!(
        LocalApplicationAuthorizationStateOps::restore_application_session_state(
            LocalApplicationAuthorizationState::export().observed_at_ns
        ),
        Err(ApplicationSessionStateError::InvalidLoginRecord),
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), persisted);
    assert_eq!(
        with_indexes(|_| ()),
        Err(ApplicationSessionStateError::IndexesUnavailable)
    );
}

#[test]
fn issuer_scoped_identity_survives_restoration_and_independent_logout() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    for device in 1..=2 {
        let (session, replay, login) =
            fixture::pair_for_browser(false, u16::try_from(device).unwrap());
        store
            .admit(session.clone(), replay.clone(), &login)
            .unwrap();
    }
    let mut ledger = restore_ledger();
    let mut budget = SessionAdmissionBudget::new(1, 1, NOW - 1).unwrap();
    let account = Principal::from_slice(&[43]);
    let created = fixture::establish_for_issuer(
        &mut store,
        &mut ledger,
        &mut budget,
        false,
        129,
        account,
        44,
    )
    .unwrap();
    let ic_auth::session::SessionEstablished::Created(session) = created else {
        panic!("another issuer owns a distinct account");
    };
    assert_eq!(
        Principal::from(session.issuer()),
        Principal::from_slice(&[44])
    );
    assert_eq!(Principal::from(session.subject()), account);
    invalidate_indexes();
    let restored = LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
    assert_eq!(restored.sessions, 3);
    assert_eq!(
        with_indexes(estimate_index_bytes).unwrap(),
        restored.estimated_index_bytes
    );

    let mut ledger = restore_ledger();
    assert!(matches!(
        ic_auth::session::logout_session(
            &mut store,
            &mut ledger,
            &mut budget,
            browser(129).into(),
            NOW
        ),
        LogoutResult::Revoked { .. },
    ));
    let original =
        fixture::authorize(&mut store, &mut ledger, NOW, 0, &["read".into()], |_| {}).unwrap();
    assert_eq!(
        Principal::from(original.issuer()),
        Principal::from_slice(&[42])
    );
    assert_eq!(Principal::from(original.subject()), account);
    invalidate_indexes();
    let restored = LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
    assert_eq!(restored.sessions, 2);

    let (session, replay, login) = fixture::pair_for_browser(false, 130);
    store.admit(session, replay, &login).unwrap();
}

#[test]
fn cleanup_rejects_invalid_login_binding_before_expiring_its_session() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store.admit(session.clone(), replay, &login).unwrap();
    let original = LocalApplicationAuthorizationState::export();
    let (_, _, other_login) = fixture::pair_for_account(false, 1, Principal::from_slice(&[44]));
    let mut invalid = original.clone();
    invalid.logins[0].1.bytes = other_login.encode().unwrap();
    // The record is canonical, but belongs to a different account than the session.
    // Removing the expired session first would hide this broken association.
    LocalApplicationAuthorizationState::import(invalid.clone());
    assert_eq!(
        LocalApplicationAuthorizationStateOps::cleanup_application_sessions(
            session.expires_at_ns()
        ),
        Err(ApplicationSessionStateError::InvalidLoginRecord)
    );
    invalid.observed_at_ns = session.expires_at_ns();
    assert_eq!(LocalApplicationAuthorizationState::export(), invalid);

    LocalApplicationAuthorizationState::import(original);
    let result = LocalApplicationAuthorizationStateOps::cleanup_application_sessions(
        session.expires_at_ns(),
    )
    .unwrap();
    assert_eq!(result.sessions_removed, 1);
    assert_eq!(LocalApplicationAuthorizationState::export().logins.len(), 1);
}

#[test]
fn cleanup_clock_survives_empty_work_restore_and_rejected_records() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    assert_eq!(
        LocalApplicationAuthorizationStateOps::cleanup_application_sessions(NOW)
            .unwrap()
            .total_removed(),
        0
    );
    let checkpoint = LocalApplicationAuthorizationState::export();
    assert_eq!(checkpoint.observed_at_ns, NOW);
    invalidate_indexes();
    LocalApplicationAuthorizationState::import(checkpoint.clone());
    LocalApplicationAuthorizationStateOps::restore_application_session_state(
        LocalApplicationAuthorizationState::export().observed_at_ns,
    )
    .unwrap();
    assert_eq!(
        LocalApplicationAuthorizationStateOps::cleanup_application_sessions(NOW - 1),
        Err(ApplicationSessionStateError::ClockRollback)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), checkpoint);
    assert_eq!(
        with_indexes(|_| ()),
        Err(ApplicationSessionStateError::IndexesUnavailable)
    );
    LocalApplicationAuthorizationStateOps::restore_application_session_state(NOW).unwrap();
    let bytes = with_indexes(|indexes| indexes.encoded_bytes).unwrap();
    assert_eq!(bytes, encoded_state_len(&checkpoint).unwrap());

    let mut invalid = checkpoint;
    invalid.login_totals.unknown = 1;
    LocalApplicationAuthorizationState::import(invalid.clone());
    assert_eq!(
        LocalApplicationAuthorizationStateOps::cleanup_application_sessions(NOW + 1),
        Err(ApplicationSessionStateError::InvalidLoginRecord)
    );
    invalid.observed_at_ns = NOW + 1;
    assert_eq!(LocalApplicationAuthorizationState::export(), invalid);
    assert_eq!(
        with_indexes(|_| ()),
        Err(ApplicationSessionStateError::IndexesUnavailable)
    );
    assert_eq!(
        LocalApplicationAuthorizationStateOps::cleanup_application_sessions(NOW),
        Err(ApplicationSessionStateError::ClockRollback)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), invalid);
}

#[test]
fn persisted_clock_fences_shared_ledger_restore_without_extending_login() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store.admit(session, replay, &login).unwrap();
    LocalApplicationAuthorizationStateOps::cleanup_application_sessions(NOW).unwrap();
    let checkpoint = LocalApplicationAuthorizationState::export();
    let restore = |now| {
        MemoryLoginRevocations::new(fixture::audience(), 4096, 256, 1_000_000_000, now)
            .unwrap()
            .restore_records(
                checkpoint.observed_at_ns,
                (
                    checkpoint.login_totals.admitted,
                    checkpoint.login_totals.unknown,
                ),
                checkpoint
                    .logins
                    .iter()
                    .map(|(_, record)| LoginRevocationChange::decode(&record.bytes).unwrap()),
            )
    };
    assert!(matches!(
        restore(NOW - 1),
        Err(ic_auth::portable::LoginRevocationError::InvalidPolicy)
    ));
    let restored = restore(NOW + 1).unwrap();
    assert_eq!(restored.observed_at_ns(), NOW + 1);
    assert_eq!(restored.occupancy(), (1, 0));
    assert_eq!(
        restored.records().next().unwrap().until_ns(),
        login.until_ns()
    );
}

#[test]
fn shared_pruning_reclaims_unknown_logouts_in_bounded_atomic_batches() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let mut ledger = fixture::ledger();
    let mut budget = SessionAdmissionBudget::new(512, 1, 0).unwrap();
    for id in 0..MAX_APPLICATION_UNKNOWN_LOGOUTS {
        assert!(matches!(
            ic_auth::session::logout_session(
                &mut store,
                &mut ledger,
                &mut budget,
                browser(u16::try_from(id + 1).unwrap()).into(),
                NOW,
            ),
            LogoutResult::Revoked { .. }
        ));
    }
    let until = ledger.records().next().unwrap().until_ns();
    let before = LocalApplicationAuthorizationState::export();
    assert_eq!(
        store
            .prune_logins(&mut ledger, until - 1, usize::MAX)
            .unwrap(),
        0
    );
    assert_eq!(
        LocalApplicationAuthorizationState::export().logins,
        before.logins
    );
    assert_eq!(
        store.prune_logins(&mut ledger, until, usize::MAX).unwrap(),
        128
    );
    assert_eq!(ledger.occupancy(), (0, 128));
    assert_eq!(
        LocalApplicationAuthorizationState::export()
            .login_totals
            .unknown,
        128
    );
    assert_eq!(
        store.prune_logins(&mut ledger, until, usize::MAX).unwrap(),
        128
    );
    assert_eq!(ledger.occupancy(), (0, 0));
    let state = LocalApplicationAuthorizationState::export();
    assert!(state.logins.is_empty());
    assert_eq!(state.login_totals.unknown, 0);
    assert_eq!(state.observed_at_ns, until);
    assert_eq!(
        with_indexes(|i| i.encoded_bytes).unwrap(),
        encoded_state_len(&state).unwrap()
    );
    assert!(matches!(
        ic_auth::session::logout_session(
            &mut store,
            &mut ledger,
            &mut budget,
            browser(300).into(),
            until
        ),
        LogoutResult::Revoked { .. }
    ));
}

#[test]
fn shared_pruning_preserves_ledger_on_failed_commit_and_retains_clock() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store.admit(session, replay, &login).unwrap();
    let mut ledger = restore_ledger();
    let until = login.until_ns();
    let mut before = LocalApplicationAuthorizationState::export();
    assert_eq!(
        store.prune_logins(&mut ledger, until, 128),
        Err(ic_auth::portable::LoginRevocationError::Storage)
    );
    before.observed_at_ns = until;
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    assert_eq!(ledger.occupancy(), (1, 0));
    assert_eq!(ledger.observed_at_ns(), until);
    assert_eq!(
        ledger.records().next().unwrap().encode().unwrap(),
        login.encode().unwrap()
    );
    assert_eq!(
        store.prune_logins(&mut ledger, until - 1, 128),
        Err(ic_auth::portable::LoginRevocationError::Clock)
    );
    LocalApplicationAuthorizationStateOps::cleanup_application_sessions(until).unwrap();
    assert_eq!(store.prune_logins(&mut ledger, until, 128).unwrap(), 1);
    assert!(
        LocalApplicationAuthorizationState::export()
            .logins
            .is_empty()
    );
    assert_eq!(ledger.occupancy(), (0, 0));
}

#[test]
fn shared_pruning_rejects_an_incomplete_runtime_ledger() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store.admit(session, replay, &login).unwrap();
    let mut ledger = fixture::ledger();
    let mut before = LocalApplicationAuthorizationState::export();
    assert_eq!(
        store.prune_logins(&mut ledger, NOW, 128),
        Err(ic_auth::portable::LoginRevocationError::Storage)
    );
    before.observed_at_ns = NOW;
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    assert_eq!(ledger.occupancy(), (0, 0));

    let mut complete = restore_ledger();
    store.fleet = fleet_key(2);
    assert_eq!(
        store.prune_logins(&mut complete, NOW, 128),
        Err(ic_auth::portable::LoginRevocationError::Storage)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    assert_eq!(complete.occupancy(), (1, 0));
}

#[test]
fn future_start_retries_original_issuance_without_extending_deadlines() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut owner = store();
    let mut ledger = fixture::ledger();
    let mut budget = SessionAdmissionBudget::new(4, 1, 0).unwrap();
    let issued = fixture::issue(false, 1, Principal::from_slice(&[43]), 42);
    let before = LocalApplicationAuthorizationState::export();
    let refusal = fixture::establish_issued(
        &mut owner,
        &mut ledger,
        &mut budget,
        &issued,
        browser(1),
        NOW - 1,
    )
    .unwrap_err();
    assert_eq!(
        refusal,
        SessionError::Claims(ic_auth::portable::PortableClaimsError::NotYetValid)
    );
    assert_eq!(refusal.admission_refusal(), AdmissionRefusal::NotYetValid);
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    assert_eq!(ledger.occupancy(), (0, 0));
    let established = fixture::establish_issued(
        &mut owner,
        &mut ledger,
        &mut budget,
        &issued,
        browser(1),
        NOW,
    )
    .unwrap();
    let ic_auth::session::SessionEstablished::Created(session) = established else {
        panic!("first valid attempt must create the session");
    };
    let committed = LocalApplicationAuthorizationState::export();
    // Retry recovers the existing session, even after the admission window closes.
    // It does not extend the session or admit a new receiver.
    assert_eq!(
        fixture::establish_issued(
            &mut owner,
            &mut ledger,
            &mut budget,
            &issued,
            browser(1),
            NOW + 2_000_000_000
        ),
        Ok(ic_auth::session::SessionEstablished::ExactRetry(session))
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), committed);
    {
        let _fresh = ApplicationSessionTestStateGuard::empty();
        let mut fresh_ledger = fixture::ledger();
        let refusal = fixture::establish_issued(
            &mut store(),
            &mut fresh_ledger,
            &mut budget,
            &issued,
            browser(1),
            NOW + 2_000_000_000,
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            SessionError::Claims(ic_auth::portable::PortableClaimsError::Expired)
        );
        assert_eq!(refusal.admission_refusal(), AdmissionRefusal::Expired);
        assert!(
            LocalApplicationAuthorizationState::export()
                .sessions
                .is_empty()
        );
        assert_eq!(fresh_ledger.occupancy(), (0, 0));
    }
    assert_eq!(LocalApplicationAuthorizationState::export(), committed);
}

#[test]
fn startup_clock_rollback_closes_indexes_and_preserves_original_leases() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut owner = store();
    let (session, replay, login) = pair(false);
    owner.admit(session.clone(), replay, &login).unwrap();
    LocalApplicationAuthorizationStateOps::cleanup_application_sessions(NOW).unwrap();
    let mut checkpoint = LocalApplicationAuthorizationState::export();
    assert_eq!(
        LocalApplicationAuthorizationStateOps::restore_application_session_state(NOW - 1),
        Err(ApplicationSessionStateError::ClockRollback)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), checkpoint);
    assert_eq!(
        with_indexes(|_| ()),
        Err(ApplicationSessionStateError::IndexesUnavailable)
    );
    let restored =
        LocalApplicationAuthorizationStateOps::restore_application_session_state(NOW + 1).unwrap();
    checkpoint.observed_at_ns = NOW + 1;
    assert_eq!(LocalApplicationAuthorizationState::export(), checkpoint);
    assert_eq!((restored.sessions, restored.replays), (1, 1));
    assert_eq!(
        restored.stable_bytes,
        encoded_state_len(&checkpoint).unwrap()
    );
    owner
        .read(|tx| {
            assert_eq!(tx.session(session.caller())?.as_deref(), Some(&session));
            Ok(())
        })
        .unwrap();
}

#[test]
fn account_and_other_device_cannot_substitute_for_credential_browser() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut owner = store();
    let mut ledger = fixture::ledger();
    let mut budget = SessionAdmissionBudget::new(4, 1, 0).unwrap();
    let account = browser(2);
    let issued = fixture::issue(false, 1, account, 42);
    let before = LocalApplicationAuthorizationState::export();
    for caller in [account, browser(3)] {
        let refusal =
            fixture::establish_issued(&mut owner, &mut ledger, &mut budget, &issued, caller, NOW)
                .unwrap_err();
        assert_eq!(
            refusal,
            SessionError::Claims(ic_auth::portable::PortableClaimsError::CallerMismatch)
        );
        assert_eq!(
            refusal.admission_refusal(),
            AdmissionRefusal::InvalidCredential
        );
        assert_eq!(LocalApplicationAuthorizationState::export(), before);
        assert_eq!(ledger.occupancy(), (0, 0));
    }
    let admitted = fixture::establish_issued(
        &mut owner,
        &mut ledger,
        &mut budget,
        &issued,
        browser(1),
        NOW,
    )
    .unwrap();
    let ic_auth::session::SessionEstablished::Created(session) = admitted else {
        panic!("only the bound browser can establish a session");
    };
    assert_eq!(Principal::from(session.caller()), browser(1));
    assert_eq!(Principal::from(session.subject()), account);
    assert_eq!(
        Principal::from(session.issuer()),
        Principal::from_slice(&[42])
    );
}

#[test]
fn restored_session_requires_current_enrollment_without_rewriting_its_lease() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut owner = store();
    let mut logins = fixture::ledger();
    let mut budget = SessionAdmissionBudget::new(32, 1, 0).unwrap();
    let ic_auth::session::SessionEstablished::Created(session) = fixture::establish(
        &mut owner,
        &mut logins,
        &mut budget,
        false,
        1,
        Principal::from_slice(&[43]),
    )
    .unwrap() else {
        panic!("fresh credential must create its session");
    };
    let checkpoint = LocalApplicationAuthorizationState::export();
    invalidate_indexes();
    LocalApplicationAuthorizationStateOps::restore_application_session_state(
        checkpoint.observed_at_ns,
    )
    .unwrap();
    for variant in 0..6 {
        let mut ledger = restore_ledger();
        let actual = fixture::authorize(
            &mut owner,
            &mut ledger,
            NOW,
            0,
            &["read".into()],
            |authority| match variant {
                0 => {}
                1 => authority.issuer = Principal::from_slice(&[44]).into(),
                2 => authority.key_binding_id = [99; 32],
                3 => authority.enrollment_expires_at_ns = NOW,
                4 => authority.min_login_started_at_ns = NOW,
                _ => authority.authority_expires_at_ns = session.expires_at_ns() - 1,
            },
        );
        if variant == 0 {
            assert_eq!(
                actual.map(|value| ic_auth::session::SessionReceipt::from(&value)),
                Ok(session)
            );
        } else {
            assert_eq!(actual, Err(SessionError::StaleAuthority));
        }
        assert_eq!(LocalApplicationAuthorizationState::export(), checkpoint);
        assert_eq!(ledger.occupancy(), (1, 0));
    }
}

#[test]
fn maintenance_clock_refusal_fences_authorization_until_valid_restore() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut owner = store();
    let mut ledger = fixture::ledger();
    let mut budget = SessionAdmissionBudget::new(4, 1, 0).unwrap();
    let ic_auth::session::SessionEstablished::Created(session) = fixture::establish(
        &mut owner,
        &mut ledger,
        &mut budget,
        false,
        1,
        Principal::from_slice(&[43]),
    )
    .unwrap() else {
        panic!("fresh credential must create its session");
    };
    LocalApplicationAuthorizationStateOps::cleanup_application_sessions(NOW + 1).unwrap();
    let checkpoint = LocalApplicationAuthorizationState::export();
    assert_eq!(
        LocalApplicationAuthorizationStateOps::cleanup_application_sessions(NOW),
        Err(ApplicationSessionStateError::ClockRollback)
    );
    assert_eq!(
        fixture::authorize(
            &mut owner,
            &mut ledger,
            NOW + 1,
            0,
            &["read".into()],
            |_| {}
        ),
        Err(SessionError::StorageFailure)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), checkpoint);
    assert_eq!(ledger.occupancy(), (1, 0));

    LocalApplicationAuthorizationStateOps::restore_application_session_state(NOW + 1).unwrap();
    assert_eq!(
        fixture::authorize(
            &mut owner,
            &mut ledger,
            NOW + 1,
            0,
            &["read".into()],
            |_| {}
        )
        .map(|value| ic_auth::session::SessionReceipt::from(&value)),
        Ok(session)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), checkpoint);
    assert_eq!(ledger.occupancy(), (1, 0));
}
