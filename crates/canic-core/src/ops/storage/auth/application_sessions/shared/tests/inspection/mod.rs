//! Bounded read-only metadata projection through actual canonical session storage.
use super::*;
use canic_contracts::dto::portable_inspection::{Refusal, Request};

#[test]
fn inspection_pages_canonical_sessions_without_mutation_or_authorization() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let mut expected = Vec::new();
    for device in 1..=3 {
        let (session, replay, login) = fixture::pair_for_browser(false, device);
        expected.push(session.clone());
        store.admit(session, replay, &login).unwrap();
    }
    expected.sort_by_key(Session::caller);
    let before = LocalApplicationAuthorizationState::export();
    let mut after = None;
    for (position, session) in expected.iter().enumerate() {
        let page = LocalApplicationAuthorizationStateOps::inspect_application_sessions(
            Request { after, limit: 1 },
            NOW,
        )
        .unwrap();
        assert_eq!(page.sessions.len(), 1);
        let row = &page.sessions[0];
        assert_eq!(
            (row.caller, row.issuer, row.subject),
            (session.caller(), session.issuer(), session.subject())
        );
        assert_eq!(
            row.scopes,
            session
                .scopes()
                .iter()
                .map(|scope| scope.as_str().to_owned())
                .collect::<Vec<_>>()
        );
        assert_eq!(row.established_at_ns, session.established_at_ns());
        assert_eq!(row.expires_at_ns, session.expires_at_ns());
        assert!(!row.expired && !row.stale_generation);
        assert_eq!(page.next_after.is_some(), position < 2);
        after = page.next_after;
    }
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
    let page = LocalApplicationAuthorizationStateOps::inspect_application_sessions(
        Request {
            after: None,
            limit: 32,
        },
        expected[0].expires_at_ns(),
    )
    .unwrap();
    assert!(page.sessions.iter().all(|session| session.expired));
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
}

#[test]
fn inspection_refuses_invalid_pages_closed_indexes_and_backward_time() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    for limit in [0, 33, u16::MAX] {
        assert_eq!(
            LocalApplicationAuthorizationStateOps::inspect_application_sessions(
                Request { after: None, limit },
                NOW
            ),
            Err(Refusal::InvalidPage)
        );
    }
    LocalApplicationAuthorizationStateOps::observe_application_time(NOW).unwrap();
    assert_eq!(
        LocalApplicationAuthorizationStateOps::inspect_application_sessions(
            Request {
                after: None,
                limit: 1
            },
            NOW - 1
        ),
        Err(Refusal::Clock)
    );
    invalidate_indexes();
    assert_eq!(
        LocalApplicationAuthorizationStateOps::inspect_application_sessions(
            Request {
                after: None,
                limit: 1
            },
            NOW
        ),
        Err(Refusal::Unavailable)
    );
}

#[test]
fn inspection_marks_retained_generation_without_revalidating_authority() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut store = store();
    let (session, replay, login) = pair(false);
    store.admit(session, replay, &login).unwrap();
    let mut state = LocalApplicationAuthorizationState::export();
    state.authority_generation = 1;
    LocalApplicationAuthorizationState::import(state.clone());
    LocalApplicationAuthorizationStateOps::restore_application_session_state(NOW).unwrap();
    let page = LocalApplicationAuthorizationStateOps::inspect_application_sessions(
        Request {
            after: None,
            limit: 1,
        },
        NOW,
    )
    .unwrap();
    assert_eq!(page.generation, 1);
    assert_eq!(page.sessions[0].generation, 0);
    assert!(page.sessions[0].stale_generation);
    assert!(!page.sessions[0].expired);
    let mut expected = state;
    expected.observed_at_ns = NOW;
    assert_eq!(LocalApplicationAuthorizationState::export(), expected);
}
