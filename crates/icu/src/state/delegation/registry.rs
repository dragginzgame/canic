use crate::{Error, state::StateError};
use candid::{CandidType, Principal};
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// Delegation Registry (State Layer)
//
// Purpose:
// --------
// Provides a minimal, in-memory map of delegation sessions,
// keyed by their temporary session principal (`session_pid`).
//
// This layer is responsible **only for raw state management**:
//   * storing, retrieving, and removing `DelegationSession`s
//   * listing sessions (all or by wallet)
//   * generic retention (`retain`) for cleanup
//
// It is *not* responsible for business logic such as
// expiration policy, cleanup cadence, authorization,
// or logging — those belong in the `ops::delegation` layer.
//
// Data Model:
// -----------
// - `DelegationSession`: represents one delegation session granted by a wallet
//   * `wallet_pid` – the granting wallet principal
//   * `expires_at` – absolute expiry timestamp (seconds since epoch)
//   * `requesting_canisters` – optional tracking of who has used the session
//
// - `DelegationSessionView`: lightweight, read-only projection returned to clients
//

thread_local! {
    static DELEGATION_REGISTRY: RefCell<HashMap<Principal, DelegationSession>> =
        RefCell::new(HashMap::new());
}

///
/// DelegationRegistryError
///

#[derive(Debug, ThisError)]
pub enum DelegationRegistryError {
    #[error("no delegation found for principal '{0}'")]
    NotFound(Principal),
}

impl From<DelegationRegistryError> for Error {
    fn from(e: DelegationRegistryError) -> Self {
        StateError::from(e).into()
    }
}

///
/// DelegationSession
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct DelegationSession {
    pub wallet_pid: Principal,
    pub expires_at: u64,
    pub requesting_canisters: Vec<Principal>,
}

impl DelegationSession {
    #[must_use]
    pub const fn new(wallet_pid: Principal, expires_at: u64) -> Self {
        Self {
            wallet_pid,
            expires_at,
            requesting_canisters: Vec::new(),
        }
    }
}

///
/// DelegationSessionView
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct DelegationSessionView {
    pub wallet_pid: Principal,
    pub session_pid: Principal,
    pub expires_at: u64,
}

impl From<(Principal, &DelegationSession)> for DelegationSessionView {
    fn from((session_pid, s): (Principal, &DelegationSession)) -> Self {
        Self {
            session_pid,
            wallet_pid: s.wallet_pid,
            expires_at: s.expires_at,
        }
    }
}

///
/// RegisterSessionArgs
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct RegisterSessionArgs {
    pub session_pid: Principal,
    pub duration_secs: u64,
}

///
/// DelegationRegistry
///

pub struct DelegationRegistry;

impl DelegationRegistry {
    /// Immutable access to underlying map.
    pub fn with<R>(f: impl FnOnce(&HashMap<Principal, DelegationSession>) -> R) -> R {
        DELEGATION_REGISTRY.with_borrow(|cell| f(cell))
    }

    /// Mutable access to underlying map.
    pub fn with_mut<R>(f: impl FnOnce(&mut HashMap<Principal, DelegationSession>) -> R) -> R {
        DELEGATION_REGISTRY.with_borrow_mut(|cell| f(cell))
    }

    /// Insert or replace a session.
    pub fn insert(session_pid: Principal, session: DelegationSession) {
        Self::with_mut(|map| {
            map.insert(session_pid, session);
        });
    }

    /// Remove a session, returning the removed value if it existed.
    #[must_use]
    pub fn remove(session_pid: &Principal) -> Option<DelegationSession> {
        Self::with_mut(|map| map.remove(session_pid))
    }

    /// Get a copy of a session by its id.
    #[must_use]
    pub fn get(session_pid: &Principal) -> Option<DelegationSession> {
        Self::with(|map| map.get(session_pid).cloned())
    }

    /// List all sessions in the registry.
    #[must_use]
    pub fn list_all() -> Vec<(Principal, DelegationSession)> {
        Self::with(|map| map.iter().map(|(pid, s)| (*pid, s.clone())).collect())
    }

    /// List all sessions for a given wallet.
    #[must_use]
    pub fn list_by_wallet(wallet_pid: &Principal) -> Vec<(Principal, DelegationSession)> {
        Self::with(|map| {
            map.iter()
                .filter(|(_, s)| &s.wallet_pid == wallet_pid)
                .map(|(pid, s)| (*pid, s.clone()))
                .collect()
        })
    }

    /// Retain only entries that satisfy the given predicate.
    pub fn retain<F: FnMut(&Principal, &mut DelegationSession) -> bool>(mut f: F) {
        Self::with_mut(|map| {
            map.retain(|pid, session| f(pid, session));
        });
    }

    /// Count number of sessions.
    #[must_use]
    pub fn count() -> usize {
        Self::with(HashMap::len)
    }

    /// Clear all sessions (test-only).
    #[cfg(test)]
    pub fn clear() {
        Self::with_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_pid(n: u8) -> Principal {
        Principal::from_slice(&[n; 29])
    }

    #[test]
    fn insert_and_get_roundtrip() {
        DelegationRegistry::clear();
        let wallet = dummy_pid(1);
        let session = dummy_pid(2);
        DelegationRegistry::insert(session, DelegationSession::new(wallet, 1234));

        let found = DelegationRegistry::get(&session).unwrap();
        assert_eq!(found.wallet_pid, wallet);
        assert_eq!(found.expires_at, 1234);
    }

    #[test]
    fn remove_session() {
        DelegationRegistry::clear();
        let wallet = dummy_pid(1);
        let session = dummy_pid(2);
        DelegationRegistry::insert(session, DelegationSession::new(wallet, 1000));

        let removed = DelegationRegistry::remove(&session).unwrap();
        assert_eq!(removed.wallet_pid, wallet);
        assert!(DelegationRegistry::get(&session).is_none());
    }

    #[test]
    fn list_all_and_by_wallet() {
        DelegationRegistry::clear();
        let wallet1 = dummy_pid(1);
        let wallet2 = dummy_pid(2);
        let s1 = dummy_pid(10);
        let s2 = dummy_pid(11);

        DelegationRegistry::insert(s1, DelegationSession::new(wallet1, 1));
        DelegationRegistry::insert(s2, DelegationSession::new(wallet2, 2));

        let all = DelegationRegistry::list_all();
        assert_eq!(all.len(), 2);

        let only1 = DelegationRegistry::list_by_wallet(&wallet1);
        assert_eq!(only1.len(), 1);
        assert_eq!(only1[0].0, s1);
    }

    #[test]
    fn retain_filters_entries() {
        DelegationRegistry::clear();
        let wallet = dummy_pid(1);
        let s1 = dummy_pid(10);
        let s2 = dummy_pid(11);

        DelegationRegistry::insert(s1, DelegationSession::new(wallet, 1));
        DelegationRegistry::insert(s2, DelegationSession::new(wallet, 2));

        DelegationRegistry::retain(|pid, _| *pid == s1);

        let all = DelegationRegistry::list_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, s1);
    }

    #[test]
    fn count_tracks_number_of_sessions() {
        DelegationRegistry::clear();
        assert_eq!(DelegationRegistry::count(), 0);

        let wallet = dummy_pid(5);
        let s1 = dummy_pid(50);
        let s2 = dummy_pid(51);
        DelegationRegistry::insert(s1, DelegationSession::new(wallet, 1));
        DelegationRegistry::insert(s2, DelegationSession::new(wallet, 2));

        assert_eq!(DelegationRegistry::count(), 2);
    }
}
