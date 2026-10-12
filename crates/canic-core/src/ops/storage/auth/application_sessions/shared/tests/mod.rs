//! Durable transaction proofs using sessions issued by IC Auth 0.5.4's native
//! cryptographic fixture. These are SDK codec/storage proofs, not IC certificates
//! observed from a running canister; PocketIC owns endpoint/lifecycle qualification.

use super::*;
use crate::{
    model::auth::application_authorization::LocalApplicationAuthoritySnapshot,
    test::{seams, support::fleet_key},
};

fn pair(second: bool) -> (Session, Replay) {
    let (session, replay): (&[u8], &[u8]) = if second {
        (
            include_bytes!("fixtures/second-session.cbor"),
            include_bytes!("fixtures/second-replay.cbor"),
        )
    } else {
        (
            include_bytes!("fixtures/first-session.cbor"),
            include_bytes!("fixtures/first-replay.cbor"),
        )
    };
    (
        Session::decode(session).unwrap(),
        ciborium::from_reader(replay).unwrap(),
    )
}

fn store() -> ApplicationSessionStore {
    ApplicationSessionStore::new(&LocalApplicationAuthoritySnapshot::new(
        fleet_key(1),
        canic_contracts::ids::CanisterRole::new("app_service"),
        0,
    ))
}

#[test]
fn committed_pair_survives_restore_and_replacement_retains_consumption() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (first, first_replay) = pair(false);
    store
        .transaction(|tx| tx.stage(first.clone(), first_replay))
        .unwrap();
    let (second, second_replay) = pair(true);
    store
        .transaction(|tx| tx.stage(second.clone(), second_replay))
        .unwrap();
    let before_bytes = with_indexes(|i| i.encoded_bytes).unwrap();
    invalidate_indexes();
    let restored =
        LocalApplicationAuthorizationStateOps::restore_application_session_state().unwrap();
    assert_eq!(restored.sessions, 1);
    assert_eq!(restored.replays, 2);
    assert_eq!(restored.stable_bytes, before_bytes);
    store
        .transaction(|tx| {
            assert_eq!(tx.session(second.caller())?, Some(second.clone()));
            assert!(tx.replay_exists(first.fingerprint())?);
            assert!(tx.replay_exists(second.fingerprint())?);
            Ok(())
        })
        .unwrap();
    let before = LocalApplicationAuthorizationState::export();
    let (first, replay) = pair(false);
    assert_eq!(
        store.transaction(|tx| tx.stage(first, replay)),
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
    let (session, replay) = pair(false);
    let result: Result<(), SessionError> = store.transaction(|tx| {
        tx.stage(session, replay)?;
        Err(SessionError::StorageFailure)
    });
    assert_eq!(result, Err(SessionError::StorageFailure));
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    assert_eq!(with_indexes(|i| i.encoded_bytes).unwrap(), before_bytes);
}

#[test]
fn stored_projection_cannot_increase_shared_session_scopes() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay) = pair(false);
    store
        .transaction(|tx| tx.stage(session.clone(), replay))
        .unwrap();
    let mut state = LocalApplicationAuthorizationState::export();
    state.sessions[0].scopes.push("write".to_owned());
    LocalApplicationAuthorizationState::import(state);
    LocalApplicationAuthorizationStateOps::restore_application_session_state().unwrap();
    assert_eq!(
        store.transaction(|tx| tx.session(session.caller())),
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
    let (session, replay) = pair(false);
    assert_eq!(
        store.transaction(|tx| tx.stage(session, replay)),
        Err(SessionError::InvalidPolicyOrRecord)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);

    let mut state = before;
    state.authority_generation = 0;
    LocalApplicationAuthorizationState::import(state);
    LocalApplicationAuthorizationStateOps::restore_application_session_state().unwrap();
    // Supply the admission boundary's exhausted resource observation.
    APPLICATION_SESSION_INDEXES.with_borrow_mut(|slot| {
        slot.as_mut().unwrap().encoded_bytes = MAX_APPLICATION_SESSION_STABLE_BYTES;
    });
    let before = LocalApplicationAuthorizationState::export();
    let (session, replay) = pair(false);
    assert_eq!(
        store.transaction(|tx| tx.stage(session, replay)),
        Err(SessionError::ByteCapacity)
    );
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
}

#[test]
fn physical_replay_capacity_refuses_both_writes() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    for per_subject in [true, false] {
        let (session, replay) = pair(false);
        let bound = if per_subject {
            MAX_APPLICATION_REPLAY_RECORDS_PER_SUBJECT
        } else {
            MAX_APPLICATION_REPLAY_RECORDS
        };
        let replays = (0..bound)
            .map(|id| {
                let mut fingerprint = [0; 32];
                fingerprint[..8].copy_from_slice(&(id as u64).to_be_bytes());
                let subject = if per_subject {
                    session.subject()
                } else {
                    Principal::from_slice(&(id as u64).to_be_bytes())
                };
                LocalApplicationReplayRecord {
                    proof_fingerprint: fingerprint,
                    transport_caller: subject,
                    authenticated_subject: subject,
                    authority_generation: 0,
                    remove_at_ns: replay.remove_at_ns(),
                }
            })
            .collect();
        LocalApplicationAuthorizationState::import(LocalApplicationAuthorizationStateData {
            replays,
            ..LocalApplicationAuthorizationStateData::default()
        });
        LocalApplicationAuthorizationStateOps::restore_application_session_state().unwrap();
        let before = LocalApplicationAuthorizationState::export();
        let before_bytes = with_indexes(|i| i.encoded_bytes).unwrap();
        assert_eq!(
            store.transaction(|tx| tx.stage(session, replay)),
            Err(SessionError::Capacity)
        );
        assert_eq!(LocalApplicationAuthorizationState::export(), before);
        assert_eq!(with_indexes(|i| i.encoded_bytes).unwrap(), before_bytes);
    }
}
