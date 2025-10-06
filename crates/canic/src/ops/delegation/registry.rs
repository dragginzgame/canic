//! Business-logic wrapper around the delegation registry.
//!
//! Extends the raw [`state::delegation::DelegationRegistry`] with duration
//! constraints, logging, cleanup cadence, and helper accessors used by the
//! endpoint layer.

use crate::{
    Error, Log, log,
    ops::OpsError,
    state::delegation::{
        DelegationRegistryError, DelegationSession, DelegationSessionView, RegisterSessionArgs,
    },
    utils::time::now_secs,
};
use candid::Principal;
use std::time::Duration;

pub use crate::state::delegation::DelegationRegistry;

/// Maximum allowed session lifetime (24h).
const MAX_EXPIRATION: Duration = Duration::from_secs(24 * 60 * 60);

/// Minimum allowed session lifetime (1m).
const MIN_EXPIRATION: Duration = Duration::from_secs(60);

/// Run cleanup every N calls to `register_session`.
const CLEANUP_THRESHOLD: u64 = 1000;

/// Maximum number of requesters remembered per delegation session.
const MAX_TRACKED_REQUESTERS: usize = 32;

thread_local! {
    static CALL_COUNT: std::cell::RefCell<u64> = const { std::cell::RefCell::new(0) };
}

///
/// DelegationError
/// Errors produced by the delegation ops layer
///

#[derive(Debug, thiserror::Error)]
pub enum DelegationError {
    #[error("session length must be at least {0} seconds")]
    SessionTooShort(u64),

    #[error("session length cannot exceed {0} seconds")]
    SessionTooLong(u64),

    #[error("session expired at {0}, now={1}")]
    SessionExpired(u64, u64),

    #[error(transparent)]
    DelegationRegistryError(#[from] DelegationRegistryError),
}

impl From<DelegationError> for Error {
    fn from(err: DelegationError) -> Self {
        OpsError::from(err).into()
    }
}

///
/// DelegationRegistry
///

impl DelegationRegistry {
    /// Register a new session for a wallet.
    ///
    /// Replaces any existing sessions for the same wallet.
    /// Validates duration against min/max bounds.
    /// Returns an error if duration is invalid.
    pub fn register_session(wallet_pid: Principal, args: RegisterSessionArgs) -> Result<(), Error> {
        let duration = Duration::from_secs(args.duration_secs);

        if duration < MIN_EXPIRATION {
            return Err(DelegationError::SessionTooShort(MIN_EXPIRATION.as_secs()).into());
        }
        if duration > MAX_EXPIRATION {
            return Err(DelegationError::SessionTooLong(MAX_EXPIRATION.as_secs()).into());
        }

        // Remove any previous session(s) for this wallet.
        Self::retain(|_, s| s.wallet_pid != wallet_pid);

        // Insert the new one.
        Self::insert(
            args.session_pid,
            DelegationSession::new(wallet_pid, now_secs() + args.duration_secs),
        );

        log!(
            Log::Ok,
            "🔑 registered delegation session={}",
            args.session_pid
        );

        Self::maybe_cleanup();
        Ok(())
    }

    /// Return a read-only view of a session.
    pub fn get_view(session_pid: Principal) -> Result<DelegationSessionView, Error> {
        let s = Self::get(&session_pid).ok_or(DelegationRegistryError::NotFound(session_pid))?;

        Ok((session_pid, &s).into())
    }

    /// Resolve wallet principal from a valid session.
    pub fn resolve_wallet(session_pid: Principal) -> Result<Principal, Error> {
        let s = Self::get(&session_pid).ok_or(DelegationRegistryError::NotFound(session_pid))?;

        if s.expires_at <= now_secs() {
            return Err(DelegationError::SessionExpired(s.expires_at, now_secs()).into());
        }

        Ok(s.wallet_pid)
    }

    /// Track the requester for a session and return its current view.
    pub fn track(
        requester: Principal,
        session_pid: Principal,
    ) -> Result<DelegationSessionView, Error> {
        let mut session =
            Self::get(&session_pid).ok_or(DelegationRegistryError::NotFound(session_pid))?;
        let now = now_secs();

        if session.expires_at <= now {
            return Err(DelegationError::SessionExpired(session.expires_at, now).into());
        }

        if !session.requesting_canisters.contains(&requester) {
            if session.requesting_canisters.len() >= MAX_TRACKED_REQUESTERS {
                let drop_count = session.requesting_canisters.len() + 1 - MAX_TRACKED_REQUESTERS;
                session.requesting_canisters.drain(0..drop_count);
            }
            session.requesting_canisters.push(requester);
            Self::insert(session_pid, session.clone());
            log!(
                Log::Info,
                "👣 tracked delegation requester={requester} session={session_pid}"
            );
        }

        Ok((session_pid, &session).into())
    }

    /// List all sessions as lightweight views.
    #[must_use]
    pub fn list_all_sessions() -> Vec<DelegationSessionView> {
        Self::collect_views(|_, _| true)
    }

    /// List sessions owned by a specific wallet.
    #[must_use]
    pub fn list_sessions_by_wallet(wallet_pid: Principal) -> Vec<DelegationSessionView> {
        Self::collect_views(|_, session| session.wallet_pid == wallet_pid)
    }

    /// Revoke a session by ID, or all sessions from a wallet.
    pub fn revoke(pid: Principal) -> Result<(), Error> {
        // Try as session first.
        if Self::remove(&pid).is_some() {
            log!(Log::Info, "🗑️ revoked session={pid}");
            return Ok(());
        }

        // Otherwise, treat as wallet principal.
        let before = Self::count();
        Self::retain(|_, s| s.wallet_pid != pid);
        let after = Self::count();

        if after < before {
            log!(Log::Info, "🗑️ revoked all sessions for wallet={pid}");
            Ok(())
        } else {
            Err(DelegationRegistryError::NotFound(pid).into())
        }
    }

    /// Periodically run cleanup after many registrations.
    fn maybe_cleanup() {
        CALL_COUNT.with_borrow_mut(|count| {
            *count += 1;
            if *count % CLEANUP_THRESHOLD == 0 {
                Self::cleanup();
            }
        });
    }

    /// Remove expired sessions immediately.
    pub fn cleanup() {
        let before = Self::count();
        Self::retain(|_, s| s.expires_at > now_secs());
        let after = Self::count();

        log!(
            Log::Info,
            "🧹 cleaned up sessions, before={before}, after={after}"
        );
    }

    fn collect_views<F>(mut filter: F) -> Vec<DelegationSessionView>
    where
        F: FnMut(&Principal, &DelegationSession) -> bool,
    {
        Self::with(|map| {
            let mut views = Vec::with_capacity(map.len());
            for (pid, session) in map {
                if filter(pid, session) {
                    views.push(DelegationSessionView {
                        session_pid: *pid,
                        wallet_pid: session.wallet_pid,
                        expires_at: session.expires_at,
                    });
                }
            }
            views
        })
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::delegation::DelegationRegistry as StateDelegationRegistry;

    fn pid(n: u8) -> Principal {
        Principal::from_slice(&[n; 29])
    }

    fn reset() {
        StateDelegationRegistry::clear();
        CALL_COUNT.with_borrow_mut(|c| *c = 0);
    }

    #[test]
    fn register_and_resolve_valid_session() {
        reset();
        let wallet = pid(1);
        let session = pid(2);

        DelegationRegistry::register_session(
            wallet,
            RegisterSessionArgs {
                session_pid: session,
                duration_secs: 120,
            },
        )
        .unwrap();

        let view = DelegationRegistry::get_view(session).unwrap();
        assert_eq!(view.wallet_pid, wallet);

        let resolved = DelegationRegistry::resolve_wallet(session).unwrap();
        assert_eq!(resolved, wallet);
    }

    #[test]
    fn register_replaces_old_sessions_for_wallet() {
        reset();
        let wallet = pid(10);
        let s1 = pid(11);
        let s2 = pid(12);

        DelegationRegistry::register_session(
            wallet,
            RegisterSessionArgs {
                session_pid: s1,
                duration_secs: 120,
            },
        )
        .unwrap();
        DelegationRegistry::register_session(
            wallet,
            RegisterSessionArgs {
                session_pid: s2,
                duration_secs: 120,
            },
        )
        .unwrap();

        let all = StateDelegationRegistry::list_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, s2); // only second session remains
    }

    #[test]
    fn revoke_by_session_or_wallet() {
        reset();
        let wallet = pid(20);
        let s1 = pid(21);
        let s2 = pid(22);

        // Register first session
        DelegationRegistry::register_session(
            wallet,
            RegisterSessionArgs {
                session_pid: s1,
                duration_secs: 120,
            },
        )
        .unwrap();

        // Registering a second session replaces the first
        DelegationRegistry::register_session(
            wallet,
            RegisterSessionArgs {
                session_pid: s2,
                duration_secs: 120,
            },
        )
        .unwrap();

        // s1 is already gone, so we only need to test revoking s2
        DelegationRegistry::revoke(s2).unwrap();
        assert!(StateDelegationRegistry::get(&s2).is_none());

        // Revoke by wallet should be idempotent: no sessions left
        DelegationRegistry::revoke(wallet).unwrap_err();
    }

    #[test]
    fn cleanup_removes_expired() {
        reset();
        let wallet = pid(30);
        let expired = pid(31);
        let valid = pid(32);

        StateDelegationRegistry::insert(expired, DelegationSession::new(wallet, now_secs() - 10));
        StateDelegationRegistry::insert(valid, DelegationSession::new(wallet, now_secs() + 1000));

        DelegationRegistry::cleanup();
        let all = StateDelegationRegistry::list_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, valid);
    }

    #[test]
    fn track_records_requester_without_duplicates() {
        reset();
        let wallet = pid(40);
        let session = pid(41);
        let requester = pid(42);

        DelegationRegistry::register_session(
            wallet,
            RegisterSessionArgs {
                session_pid: session,
                duration_secs: 120,
            },
        )
        .unwrap();

        let view = DelegationRegistry::track(requester, session).unwrap();
        assert_eq!(view.session_pid, session);

        let stored = StateDelegationRegistry::get(&session).unwrap();
        assert_eq!(stored.requesting_canisters, vec![requester]);

        DelegationRegistry::track(requester, session).unwrap();
        let stored_again = StateDelegationRegistry::get(&session).unwrap();
        assert_eq!(stored_again.requesting_canisters.len(), 1);
    }

    #[test]
    fn track_trims_requester_history_when_full() {
        reset();
        let wallet = pid(50);
        let session = pid(51);

        DelegationRegistry::register_session(
            wallet,
            RegisterSessionArgs {
                session_pid: session,
                duration_secs: 120,
            },
        )
        .unwrap();

        #[allow(clippy::cast_possible_truncation)]
        for n in 0..(MAX_TRACKED_REQUESTERS + 5) {
            let requester = pid(60 + n as u8);
            DelegationRegistry::track(requester, session).unwrap();
        }

        let stored = StateDelegationRegistry::get(&session).unwrap();
        assert_eq!(stored.requesting_canisters.len(), MAX_TRACKED_REQUESTERS);

        let expected_first = pid(60 + 5); // oldest entries trimmed
        assert_eq!(
            stored.requesting_canisters.first().copied(),
            Some(expected_first)
        );
    }
}
