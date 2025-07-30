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
    static SESSION_REGISTRY: RefCell<HashMap<Principal, Session>> = RefCell::new(HashMap::new());
    static CALL_COUNT: RefCell<u64> = const { RefCell::new(0) };
}

///
/// SessionRegistry
///

#[derive(Debug, ThisError)]
pub enum SessionRegistryError {
    #[error("no expiry set for session '{0}'")]
    NoExpirySet(Principal),

    #[error("no session found for principal '{0}'")]
    NotFound(Principal),

    #[error("session expired at {0} (current time: {1})")]
    SessionExpired(u64, u64),

    #[error("session length must be at least {0} seconds")]
    SessionTooShort(u64),

    #[error("session length cannot exceed {0} seconds")]
    SessionTooLong(u64),
}

///
/// Data Structures
///

#[derive(Debug, CandidType, Deserialize, Clone)]
pub struct Session {
    wallet_pid: Principal,
    expires_at: Option<u64>,
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub struct SessionInfo {
    wallet_pid: Principal,
    session_pid: Principal,
    expires_at: Option<u64>,
    is_expired: bool,
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub struct RegisterSessionArgs {
    pub wallet_pid: Principal,
    pub session_pid: Principal,
    pub duration_secs: u64,
}

///
/// SessionRegistry
/// map of the session pid (temporary principal) to the session
///

pub struct SessionRegistry {}

impl SessionRegistry {
    /// Returns info about a specific session, including expiration status.
    pub fn get_session_info(session_pid: Principal) -> Result<SessionInfo, Error> {
        let now = now_secs();
        let session = SESSION_REGISTRY
            .with_borrow(|map| map.get(&session_pid).cloned())
            .ok_or_else(|| StateError::from(SessionRegistryError::NotFound(session_pid)))?;

        let is_expired = session.expires_at.is_none_or(|ts| now > ts);

        Ok(SessionInfo {
            wallet_pid: session.wallet_pid,
            session_pid,
            expires_at: session.expires_at,
            is_expired,
        })
    }

    /// Resolves the wallet (grantor) associated with a valid, non-expired session.
    pub fn resolve_wallet(caller: Principal) -> Result<Principal, Error> {
        let now = now_secs();

        // Check if session exists
        let session = SESSION_REGISTRY
            .with_borrow(|map| map.get(&caller).cloned())
            .ok_or_else(|| StateError::from(SessionRegistryError::NotFound(caller)))?;

        // Check it has a valid expiry
        let expires_at = session
            .expires_at
            .ok_or_else(|| StateError::from(SessionRegistryError::NoExpirySet(caller)))?;

        // Check it has expired
        if now > expires_at {
            return Err(
                StateError::from(SessionRegistryError::SessionExpired(expires_at, now)).into(),
            );
        }

        Ok(session.wallet_pid)
    }

    /// Lists all sessions currently in the registry.
    #[must_use]
    pub fn list_all_sessions() -> Vec<SessionInfo> {
        let now = now_secs();

        SESSION_REGISTRY.with_borrow(|map| {
            map.iter()
                .map(|(&s, d)| SessionInfo {
                    session_pid: s,
                    wallet_pid: d.wallet_pid,
                    expires_at: d.expires_at,
                    is_expired: d.expires_at.is_none_or(|expiry| now > expiry),
                })
                .collect()
        })
    }

    /// Lists all sessions associated with the given wallet principal.
    #[must_use]
    pub fn list_sessions_by_wallet(wallet_pid: Principal) -> Vec<SessionInfo> {
        let now = now_secs();

        SESSION_REGISTRY.with_borrow(|map| {
            map.iter()
                .filter(|(_, del)| del.wallet_pid == wallet_pid)
                .map(|(&session_pid, del)| {
                    let is_expired = del.expires_at.is_none_or(|expiry| now > expiry);

                    SessionInfo {
                        session_pid,
                        wallet_pid: del.wallet_pid,
                        expires_at: del.expires_at,
                        is_expired,
                    }
                })
                .collect()
        })
    }

    /// Registers a new session for a wallet with a limited duration.
    /// Removes any previous session associated with the same wallet.
    pub fn register_session(args: RegisterSessionArgs) -> Result<(), Error> {
        let duration = Duration::from_secs(args.duration_secs);

        // Validate expiration time
        if duration < MIN_EXPIRATION {
            Err(StateError::from(SessionRegistryError::SessionTooShort(
                MIN_EXPIRATION.as_secs(),
            )))?;
        }

        if duration > MAX_EXPIRATION {
            Err(StateError::from(SessionRegistryError::SessionTooLong(
                MAX_EXPIRATION.as_secs(),
            )))?;
        }

        let expires_at = now_secs() + args.duration_secs;

        SESSION_REGISTRY.with_borrow_mut(|map| {
            // Remove any existing session from the same wallet
            map.retain(|_, session| session.wallet_pid != args.wallet_pid);

            // Insert the new session
            map.insert(
                args.session_pid,
                Session {
                    wallet_pid: args.wallet_pid,
                    expires_at: Some(expires_at),
                },
            );
        });

        // Periodically clean up expired sessions (every CLEANUP_THRESHOLD calls)
        CALL_COUNT.with_borrow_mut(|count| {
            *count += 1;
            if *count % CLEANUP_THRESHOLD == 0 {
                Self::cleanup();
            }
        });

        Ok(())
    }

    ///
    /// Revokes a session or all sessions granted by a wallet.
    ///
    /// - If `pid` matches a known session ID (i.e., a `Principal` that was delegated),
    ///   that specific session will be removed.
    /// - Otherwise, if `pid` matches a grantor (wallet principal), all sessions
    ///   granted by that wallet will be revoked.
    /// - Returns an error if no matching session or grantor is found.
    ///
    pub fn revoke_sessions_by_wallet(wallet_pid: Principal) -> Result<(), Error> {
        let removed_count = SESSION_REGISTRY.with_borrow_mut(|map| {
            let before = map.len();
            map.retain(|_, s| s.wallet_pid != wallet_pid);
            before - map.len()
        });

        if removed_count > 0 {
            Ok(())
        } else {
            Err(StateError::from(SessionRegistryError::NotFound(wallet_pid)).into())
        }
    }

    /// Revokes a specific session by its session ID.
    pub fn revoke_session_by_id(session_id: Principal) -> Result<(), Error> {
        let removed = SESSION_REGISTRY.with_borrow_mut(|map| map.remove(&session_id));

        if removed.is_some() {
            Ok(())
        } else {
            Err(StateError::from(SessionRegistryError::NotFound(session_id)).into())
        }
    }

    /// Removes all expired sessions from the registry.
    fn cleanup() {
        let before = SESSION_REGISTRY.with_borrow(|map| map.len());

        let now = now_secs();
        SESSION_REGISTRY.with_borrow_mut(|map| {
            map.retain(|_, d| d.expires_at.is_some_and(|ts| now <= ts));
        });

        let after = SESSION_REGISTRY.with_borrow(|map| map.len());

        log!(
            Log::Info,
            "cleaned up sessions, before: {before}, after: {after}"
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_pid(n: u8) -> Principal {
        Principal::from_slice(&[n; 29])
    }

    fn reset_state() {
        SESSION_REGISTRY.with_borrow_mut(|map| map.clear());
        CALL_COUNT.with_borrow_mut(|count| *count = 0);
    }

    #[test]
    fn register_and_query_session_view() {
        reset_state();
        let wallet = dummy_pid(1);
        let session = dummy_pid(2);

        SessionRegistry::register_session(RegisterSessionArgs {
            wallet_pid: wallet,
            session_pid: session,
            duration_secs: 300,
        })
        .unwrap();

        let view = SessionRegistry::get_session_info(session).unwrap();
        assert_eq!(view.wallet_pid, wallet);
        assert_eq!(view.session_pid, session);
        assert!(!view.is_expired);
    }

    #[test]
    fn expired_session_should_be_cleaned_up() {
        reset_state();
        let wallet = dummy_pid(3);
        let session = dummy_pid(4);

        SESSION_REGISTRY.with_borrow_mut(|map| {
            map.insert(
                session,
                Session {
                    wallet_pid: wallet,
                    expires_at: Some(now_secs() - 10), // already expired
                },
            );
        });

        SessionRegistry::cleanup();
        let found = SESSION_REGISTRY.with_borrow(|map| map.contains_key(&session));
        assert!(!found);
    }

    #[test]
    fn revoke_specific_session_removes_only_target() {
        reset_state();
        let wallet1 = dummy_pid(8);
        let wallet2 = dummy_pid(9);
        let session1 = dummy_pid(10);
        let session2 = dummy_pid(11);

        SessionRegistry::register_session(RegisterSessionArgs {
            wallet_pid: wallet1,
            session_pid: session1,
            duration_secs: 1000,
        })
        .unwrap();

        SessionRegistry::register_session(RegisterSessionArgs {
            wallet_pid: wallet2,
            session_pid: session2,
            duration_secs: 1000,
        })
        .unwrap();

        SessionRegistry::revoke_session_by_id(session1).unwrap();

        let still_exists = SESSION_REGISTRY.with_borrow(|map| map.contains_key(&session2));
        assert!(still_exists);

        let removed = SESSION_REGISTRY.with_borrow(|map| !map.contains_key(&session1));
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
            SessionRegistry::register_session(RegisterSessionArgs {
                wallet_pid,
                session_pid,
                duration_secs: 1000,
            })
            .unwrap();
        }

        SessionRegistry::revoke_sessions_by_wallet(wallet).unwrap();

        let keys = SESSION_REGISTRY.with_borrow(|map| map.keys().copied().collect::<Vec<_>>());
        assert!(!keys.contains(&s1));
        assert!(!keys.contains(&s2));
        assert!(keys.contains(&s3));
    }

    #[test]
    fn view_contains_expired_flag() {
        reset_state();
        let wallet = dummy_pid(20);
        let session = dummy_pid(21);

        SESSION_REGISTRY.with_borrow_mut(|map| {
            map.insert(
                session,
                Session {
                    wallet_pid: wallet,
                    expires_at: Some(now_secs() - 1), // expired
                },
            );
        });

        let view = SessionRegistry::get_session_info(session).unwrap();
        assert!(view.is_expired);
    }
}
