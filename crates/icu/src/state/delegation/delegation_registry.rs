use crate::{Error, Log, log, state::StateError, utils::time::now_secs};
use candid::{CandidType, Principal};
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap, time::Duration};
use thiserror::Error as ThisError;

///
/// Constants for validation
///

const MAX_EXPIRATION: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours
const MIN_EXPIRATION: Duration = Duration::from_secs(60); // 1 minute
const CLEANUP_THRESHOLD: u64 = 1000;

//
// SESSIONS
//

thread_local! {
    static DELEGATION_REGISTRY: RefCell<HashMap<Principal, DelegationSession>> = RefCell::new(HashMap::new());
    static CALL_COUNT: RefCell<u64> = const { RefCell::new(0) };
}

///
/// DelegationRegistry
///

#[derive(Debug, ThisError)]
pub enum DelegationRegistryError {
    #[error("no delegation found for principal '{0}'")]
    NotFound(Principal),

    #[error("session expired at {0} (current time: {1})")]
    SessionExpired(u64, u64),

    #[error("session length must be at least {0} seconds")]
    SessionTooShort(u64),

    #[error("session length cannot exceed {0} seconds")]
    SessionTooLong(u64),
}

///
/// DelegationSession
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct DelegationSession {
    wallet_pid: Principal,
    expires_at: u64,
    requesting_canisters: Vec<Principal>,
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

    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at < now_secs()
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
    pub is_expired: bool,
}

impl From<(Principal, &DelegationSession)> for DelegationSessionView {
    fn from((session_pid, session): (Principal, &DelegationSession)) -> Self {
        Self {
            session_pid,
            wallet_pid: session.wallet_pid,
            expires_at: session.expires_at,
            is_expired: session.is_expired(),
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
/// map of the session pid (temporary principal) to the session
///

pub struct DelegationRegistry {}

impl DelegationRegistry {
    //
    // INTERNAL ACCESSORS
    //

    pub fn with<R>(f: impl FnOnce(&HashMap<Principal, DelegationSession>) -> R) -> R {
        DELEGATION_REGISTRY.with_borrow(|cell| f(cell))
    }

    pub fn with_mut<R>(f: impl FnOnce(&mut HashMap<Principal, DelegationSession>) -> R) -> R {
        DELEGATION_REGISTRY.with_borrow_mut(|cell| f(cell))
    }

    //
    // METHODS
    //

    #[must_use]
    pub fn is_empty() -> bool {
        Self::with(HashMap::is_empty)
    }

    /// Returns info about a specific session, including expiration status.
    pub fn get(session_pid: Principal) -> Result<DelegationSessionView, Error> {
        let session = Self::with(|map| map.get(&session_pid).cloned())
            .ok_or_else(|| StateError::from(DelegationRegistryError::NotFound(session_pid)))?;

        Ok((session_pid, &session).into())
    }

    pub fn track(
        caller: Principal,
        session_pid: Principal,
    ) -> Result<DelegationSessionView, Error> {
        Self::with_mut(|map| {
            let session = map
                .get_mut(&session_pid)
                .ok_or_else(|| StateError::from(DelegationRegistryError::NotFound(session_pid)))?;

            if !session.requesting_canisters.contains(&caller) {
                session.requesting_canisters.push(caller);
            }

            Ok(DelegationSessionView::from((session_pid, &*session)))
        })
    }

    /// Resolves the wallet (grantor) associated with a valid, non-expired session.
    pub fn resolve_wallet(caller: Principal) -> Result<Principal, Error> {
        // Check if session exists
        let session = Self::with(|map| map.get(&caller).cloned())
            .ok_or_else(|| StateError::from(DelegationRegistryError::NotFound(caller)))?;

        // Check it has expired
        if session.is_expired() {
            return Err(StateError::from(DelegationRegistryError::SessionExpired(
                session.expires_at,
                now_secs(),
            ))
            .into());
        }

        Ok(session.wallet_pid)
    }

    /// Lists all sessions currently in the registry.
    #[must_use]
    pub fn list_all_sessions() -> Vec<DelegationSessionView> {
        Self::with(|map| {
            map.iter()
                .map(|(&pid, session)| (pid, session).into())
                .collect()
        })
    }

    /// Lists all sessions associated with the given wallet principal.
    #[must_use]
    pub fn list_sessions_by_wallet(wallet_pid: Principal) -> Vec<DelegationSessionView> {
        Self::with(|map| {
            map.iter()
                .filter(|(_, session)| session.wallet_pid == wallet_pid)
                .map(|(&pid, session)| (pid, session).into())
                .collect()
        })
    }

    /// Registers a new session for a wallet with a limited duration.
    /// Removes any previous session associated with the same wallet.
    /// This call is expected to come from the front end
    pub fn register_session(wallet_pid: Principal, args: RegisterSessionArgs) -> Result<(), Error> {
        let duration = Duration::from_secs(args.duration_secs);

        // Validate expiration time
        if duration < MIN_EXPIRATION {
            Err(StateError::from(DelegationRegistryError::SessionTooShort(
                MIN_EXPIRATION.as_secs(),
            )))?;
        }

        if duration > MAX_EXPIRATION {
            Err(StateError::from(DelegationRegistryError::SessionTooLong(
                MAX_EXPIRATION.as_secs(),
            )))?;
        }

        Self::with_mut(|map| {
            // Remove any existing session from the same wallet
            map.retain(|_, session| session.wallet_pid != wallet_pid);

            // Insert the new session
            map.insert(
                args.session_pid,
                DelegationSession {
                    wallet_pid,
                    expires_at: now_secs() + args.duration_secs,
                    requesting_canisters: Vec::new(),
                },
            );
        });

        // Periodically clean up expired sessions (every CLEANUP_THRESHOLD calls)
        Self::maybe_cleanup();

        Ok(())
    }

    ///
    /// Revokes a session or all sessions granted by a wallet principal.
    ///
    /// - If `pid` matches a registered session, that session is removed.
    /// - Otherwise, removes all sessions issued by `pid` (wallet).
    ///
    /// Returns an error if no matching entry is found.
    ///
    pub fn revoke_session_or_wallet(pid: Principal) -> Result<(), Error> {
        // 1. Try as session ID
        if Self::with(|map| map.contains_key(&pid)) {
            Self::with_mut(|map| {
                map.remove(&pid);
            });

            return Ok(());
        }

        // 2. Try as wallet grantor
        let removed = Self::with_mut(|map| {
            let original_len = map.len();
            map.retain(|_, s| s.wallet_pid != pid);
            original_len - map.len()
        });

        if removed > 0 {
            Ok(())
        } else {
            Err(StateError::from(DelegationRegistryError::NotFound(pid)).into())
        }
    }

    // maybe_cleanup
    fn maybe_cleanup() {
        CALL_COUNT.with_borrow_mut(|count| {
            *count += 1;
            if *count % CLEANUP_THRESHOLD == 0 {
                Self::cleanup();
            }
        });
    }

    /// Removes all expired sessions from the registry.
    fn cleanup() {
        let before = Self::with(HashMap::len);

        // retain the ones not expired
        Self::with_mut(|map| {
            map.retain(|_, s| !s.is_expired());
        });

        let after = Self::with(HashMap::len);

        log!(
            Log::Info,
            "cleaned up sessions, before: {before}, after: {after}"
        );
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

    fn reset_state() {
        DELEGATION_REGISTRY.with_borrow_mut(HashMap::clear);
        CALL_COUNT.with_borrow_mut(|count| *count = 0);
    }

    #[test]
    fn register_and_query_session_view() {
        reset_state();
        let wallet = dummy_pid(1);
        let session = dummy_pid(2);

        DelegationRegistry::register_session(
            wallet,
            RegisterSessionArgs {
                session_pid: session,
                duration_secs: 300,
            },
        )
        .unwrap();

        let view = DelegationRegistry::get(session).unwrap();
        assert_eq!(view.wallet_pid, wallet);
        assert_eq!(view.session_pid, session);
        assert!(!view.is_expired);
    }

    #[test]
    fn expired_session_should_be_cleaned_up() {
        reset_state();
        let wallet = dummy_pid(3);
        let session = dummy_pid(4);

        DELEGATION_REGISTRY.with_borrow_mut(|map| {
            map.insert(session, DelegationSession::new(wallet, now_secs() - 10));
        });

        DelegationRegistry::cleanup();
        let found = DELEGATION_REGISTRY.with_borrow(|map| map.contains_key(&session));
        assert!(!found);
    }

    #[test]
    fn revoke_specific_session_removes_only_target() {
        reset_state();
        let wallet1 = dummy_pid(8);
        let wallet2 = dummy_pid(9);
        let session1 = dummy_pid(10);
        let session2 = dummy_pid(11);

        DelegationRegistry::register_session(
            wallet1,
            RegisterSessionArgs {
                session_pid: session1,
                duration_secs: 1000,
            },
        )
        .unwrap();

        DelegationRegistry::register_session(
            wallet2,
            RegisterSessionArgs {
                session_pid: session2,
                duration_secs: 1000,
            },
        )
        .unwrap();

        DelegationRegistry::revoke_session_or_wallet(session1).unwrap();

        let still_exists = DELEGATION_REGISTRY.with_borrow(|map| map.contains_key(&session2));
        assert!(still_exists);

        let removed = DELEGATION_REGISTRY.with_borrow(|map| !map.contains_key(&session1));
        assert!(removed);
    }

    #[test]
    fn revoke_all_sessions_from_wallet_removes_them_all() {
        reset_state();
        let wallet = dummy_pid(11);
        let s1 = dummy_pid(12);
        let s2 = dummy_pid(13);
        let s3 = dummy_pid(99); // different wallet

        for (wallet_pid, session_pid) in [(wallet, s1), (wallet, s2), (dummy_pid(200), s3)] {
            DelegationRegistry::register_session(
                wallet_pid,
                RegisterSessionArgs {
                    session_pid,
                    duration_secs: 1000,
                },
            )
            .unwrap();
        }

        DelegationRegistry::revoke_session_or_wallet(wallet).unwrap();

        let keys = DELEGATION_REGISTRY.with_borrow(|map| map.keys().copied().collect::<Vec<_>>());
        assert!(!keys.contains(&s1));
        assert!(!keys.contains(&s2));
        assert!(keys.contains(&s3));
    }

    #[test]
    fn view_contains_expired_flag() {
        reset_state();
        let wallet = dummy_pid(20);
        let session = dummy_pid(21);

        DELEGATION_REGISTRY.with_borrow_mut(|map| {
            map.insert(session, DelegationSession::new(wallet, now_secs() - 1))
        });

        let view = DelegationRegistry::get(session).unwrap();
        assert!(view.is_expired);
    }
}
