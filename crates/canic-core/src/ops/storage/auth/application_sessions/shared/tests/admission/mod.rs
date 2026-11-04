//! Actual stable-store count boundaries through shared Auth admission validation.

use super::*;

#[test]
fn session_count_saturation_allows_replacement_without_admitting_another_caller() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    for device in 1..=MAX_ACTIVE_APPLICATION_SESSIONS {
        let (session, replay, login) =
            fixture::pair_for_browser(false, u16::try_from(device).unwrap());
        assert_eq!(store.admit(session, replay, &login), Ok(false));
    }
    let before = LocalApplicationAuthorizationState::export();
    let (session, replay, login) = fixture::pair_for_browser(
        false,
        u16::try_from(MAX_ACTIVE_APPLICATION_SESSIONS + 1).unwrap(),
    );
    assert_eq!(
        store.admit(session, replay, &login),
        Err(SessionError::Capacity)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);

    let (replacement, replay, login) = pair(true);
    assert_eq!(store.admit(replacement, replay, &login), Ok(true));
    let after = LocalApplicationAuthorizationState::export();
    assert_eq!(after.sessions.len(), MAX_ACTIVE_APPLICATION_SESSIONS);
    assert_eq!(after.replays.len(), before.replays.len() + 1);
    assert_eq!(after.login_totals, before.login_totals);
    assert_saturated_receiver_remains_usable(&mut store);
}

// Capacity must stop new admissions without disabling retained authorization,
// exact retries or durable logout. Rebuild the real indexes and login ledger.
fn assert_saturated_receiver_remains_usable(owner: &mut ApplicationSessionStore) {
    let checkpoint = LocalApplicationAuthorizationState::export();
    invalidate_indexes();
    LocalApplicationAuthorizationStateOps::restore_application_session_state(NOW).unwrap();
    let mut ledger = restore_ledger();
    let mut budget = SessionAdmissionBudget::new(4, 1, 0).unwrap();
    let issued = fixture::issue(true, 1, Principal::from_slice(&[43]), 42);
    let authorized =
        fixture::authorize(owner, &mut ledger, NOW, 0, &["read".into()], |_| {}).unwrap();
    assert_eq!(
        fixture::establish_issued(owner, &mut ledger, &mut budget, &issued, browser(1), NOW),
        Ok(ic_auth::session::SessionEstablished::ExactRetry(
            ic_auth::session::SessionReceipt::from(&authorized)
        ))
    );
    let mut restored = checkpoint;
    restored.observed_at_ns = NOW;
    assert_eq!(LocalApplicationAuthorizationState::export(), restored);
    assert!(matches!(
        ic_auth::session::logout_session(owner, &mut ledger, &mut budget, browser(1).into(), NOW),
        LogoutResult::Revoked { .. }
    ));
    let logged_out = LocalApplicationAuthorizationState::export();
    assert_eq!(
        logged_out.sessions.len(),
        MAX_ACTIVE_APPLICATION_SESSIONS - 1
    );
    assert_eq!(logged_out.replays, restored.replays);
    assert_eq!(logged_out.login_totals, restored.login_totals);
    invalidate_indexes();
    LocalApplicationAuthorizationStateOps::restore_application_session_state(NOW).unwrap();
    let mut ledger = restore_ledger();
    assert!(
        fixture::authorize(owner, &mut ledger, NOW, 0, &["read".into()], |authority| {
            authority.caller = browser(2).into();
        })
        .is_ok()
    );
    assert_eq!(
        fixture::authorize(owner, &mut ledger, NOW, 0, &["read".into()], |_| {}),
        Err(SessionError::Revocation(
            ic_auth::portable::LoginRevocationError::Revoked
        ))
    );
    assert_eq!(
        fixture::establish_issued(owner, &mut ledger, &mut budget, &issued, browser(1), NOW),
        Err(SessionError::Revocation(
            ic_auth::portable::LoginRevocationError::Revoked
        ))
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), logged_out);
    let device = u16::try_from(MAX_ACTIVE_APPLICATION_SESSIONS + 1).unwrap();
    let (session, replay, login) = fixture::pair_for_browser(false, device);
    assert_eq!(owner.admit(session, replay, &login), Ok(false));
    let admitted = LocalApplicationAuthorizationState::export();
    assert_eq!(admitted.sessions.len(), MAX_ACTIVE_APPLICATION_SESSIONS);
    assert_eq!(admitted.replays.len(), logged_out.replays.len() + 1);
    assert_eq!(
        admitted.login_totals.admitted,
        logged_out.login_totals.admitted + 1
    );
}

#[test]
fn physical_replay_caps_preserve_prior_records_and_isolate_caller_capacity() {
    let _lock = seams::lock();
    for global in [false, true] {
        let _state = ApplicationSessionTestStateGuard::empty();
        let mut retained = LocalApplicationAuthorizationStateData::default();
        let count = if global {
            MAX_APPLICATION_REPLAY_RECORDS
        } else {
            MAX_APPLICATION_REPLAY_RECORDS_PER_CALLER
        };
        for index in 0..count {
            let mut fingerprint = [0; 32];
            fingerprint[..8].copy_from_slice(&u64::try_from(index + 1).unwrap().to_be_bytes());
            let device = if global {
                1 + index / (MAX_APPLICATION_REPLAY_RECORDS_PER_CALLER / 2)
            } else {
                1
            };
            retained.replays.push(LocalApplicationReplayRecord {
                proof_fingerprint: fingerprint,
                transport_caller: browser(u16::try_from(device).unwrap()),
                authority_generation: 0,
                remove_at_ns: NOW + 1,
            });
        }
        LocalApplicationAuthorizationState::import(retained);
        LocalApplicationAuthorizationStateOps::restore_application_session_state(0).unwrap();
        let before = LocalApplicationAuthorizationState::export();
        let mut store = store();
        let (session, replay, login) = pair(false);
        assert_eq!(
            store.admit(session, replay, &login),
            Err(SessionError::Capacity)
        );
        assert_eq!(LocalApplicationAuthorizationState::export(), before);
        if !global {
            let (session, replay, login) = fixture::pair_for_browser(false, 2);
            assert_eq!(store.admit(session, replay, &login), Ok(false));
        }
    }
}

#[test]
fn configured_receiver_capacity_preserves_retained_state_on_smaller_restore() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    LocalApplicationAuthorizationStateOps::configure_receiver_capacity(2).unwrap();
    LocalApplicationAuthorizationStateOps::restore_application_session_state(NOW).unwrap();
    let mut owner = store();
    for device in 1..=2 {
        let (session, replay, login) = fixture::pair_for_browser(false, device);
        owner.admit(session, replay, &login).unwrap();
    }
    let checkpoint = LocalApplicationAuthorizationState::export();
    let (session, replay, login) = fixture::pair_for_browser(false, 3);
    assert_eq!(
        owner.admit(session, replay, &login),
        Err(SessionError::Capacity)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), checkpoint);
    LocalApplicationAuthorizationStateOps::configure_receiver_capacity(1).unwrap();
    assert_eq!(
        LocalApplicationAuthorizationStateOps::restore_application_session_state(NOW),
        Err(ApplicationSessionStateError::ActiveGlobalCapacity)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), checkpoint);
    assert_eq!(
        owner.read(|tx| tx.session(browser(1).into()).map(|_| ())),
        Err(SessionError::StorageFailure)
    );
    LocalApplicationAuthorizationStateOps::configure_receiver_capacity(4).unwrap();
    let stats =
        LocalApplicationAuthorizationStateOps::restore_application_session_state(NOW).unwrap();
    assert_eq!((stats.sessions, stats.replays), (2, 2));
    assert_eq!(LocalApplicationAuthorizationState::export(), checkpoint);
    let (session, replay, login) = fixture::pair_for_browser(false, 3);
    assert_eq!(owner.admit(session, replay, &login), Ok(false));
    assert_eq!(
        LocalApplicationAuthorizationStateOps::configure_receiver_capacity(0),
        Err(ApplicationSessionStateError::InvalidCapacity)
    );
    assert_eq!(
        owner.read(|tx| tx.session(browser(1).into()).map(|_| ())),
        Err(SessionError::StorageFailure)
    );
}
