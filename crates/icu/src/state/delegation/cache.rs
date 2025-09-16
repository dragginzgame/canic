use crate::{state::delegation::DelegationSessionView, utils::time::now_secs};
use candid::Principal;
use std::{cell::RefCell, collections::HashMap};

//
// Delegation Cache (State Layer)
//
// Purpose:
// --------
// Provides a lightweight, in-memory cache of `DelegationSessionView`
// entries keyed by their session principal.
//
// Characteristics:
// - Ephemeral: cleared on canister upgrade.
// - No policy decisions: only raw insert/get/remove/cleanup.
// - Intended to speed up lookups or reduce recomputation.
//
// Responsibilities:
// - Insert and remove cached session views
// - List and count cache entries
// - Periodically clean up expired entries
//
// Not responsible for:
// - Logging
// - Expiration cadence or thresholds (handled in `ops::delegation`)
//

thread_local! {
    static DELEGATION_CACHE: RefCell<HashMap<Principal, DelegationSessionView>> =
        RefCell::new(HashMap::new());
}

///
/// DelegationCache
/// raw state API for delegation session caching.
///

pub struct DelegationCache;

impl DelegationCache {
    /// Returns true if cache is empty.
    #[must_use]
    pub fn is_empty() -> bool {
        DELEGATION_CACHE.with_borrow(HashMap::is_empty)
    }

    /// Lookup a cached session by principal.
    #[must_use]
    pub fn get(session_pid: Principal) -> Option<DelegationSessionView> {
        DELEGATION_CACHE.with_borrow(|map| map.get(&session_pid).cloned())
    }

    /// Insert or replace a cached session.
    pub fn insert(session_pid: Principal, session: DelegationSessionView) {
        DELEGATION_CACHE.with_borrow_mut(|map| {
            map.insert(session_pid, session);
        });
    }

    /// Remove a cached session, returning true if it existed.
    #[must_use]
    pub fn remove(session_pid: Principal) -> bool {
        DELEGATION_CACHE.with_borrow_mut(|map| map.remove(&session_pid).is_some())
    }

    /// Return all cached sessions.
    #[must_use]
    pub fn list() -> Vec<(Principal, DelegationSessionView)> {
        DELEGATION_CACHE.with_borrow(|map| map.iter().map(|(k, v)| (*k, v.clone())).collect())
    }

    /// Remove all expired sessions.
    /// Returns `(before, after)` counts for metrics or logging.
    pub fn cleanup_expired() -> (usize, usize) {
        let now = now_secs();
        let before = DELEGATION_CACHE.with_borrow(HashMap::len);
        DELEGATION_CACHE.with_borrow_mut(|map| {
            map.retain(|_, s| s.expires_at > now);
        });
        let after = DELEGATION_CACHE.with_borrow(HashMap::len);
        (before, after)
    }

    /// Return number of cached sessions.
    #[must_use]
    pub fn count() -> usize {
        DELEGATION_CACHE.with_borrow(HashMap::len)
    }

    /// Clear cache (for tests only).
    #[cfg(test)]
    pub fn clear() {
        DELEGATION_CACHE.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::delegation::DelegationSessionView;

    fn pid(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn make_view(wallet: Principal, session: Principal, expires_at: u64) -> DelegationSessionView {
        DelegationSessionView {
            wallet_pid: wallet,
            session_pid: session,
            expires_at,
        }
    }

    #[test]
    fn insert_and_get_roundtrip() {
        DelegationCache::clear();
        let wallet = pid(1);
        let session = pid(2);
        let view = make_view(wallet, session, now_secs() + 100);

        DelegationCache::insert(session, view);
        let found = DelegationCache::get(session).unwrap();

        assert_eq!(found.session_pid, session);
        assert_eq!(found.wallet_pid, wallet);
    }

    #[test]
    fn remove_entry() {
        DelegationCache::clear();
        let session = pid(10);
        let view = make_view(pid(1), session, now_secs() + 100);
        DelegationCache::insert(session, view);

        assert!(DelegationCache::remove(session));
        assert!(DelegationCache::get(session).is_none());
    }

    #[test]
    fn list_and_count() {
        DelegationCache::clear();
        let s1 = pid(11);
        let s2 = pid(12);

        DelegationCache::insert(s1, make_view(pid(1), s1, now_secs() + 100));
        DelegationCache::insert(s2, make_view(pid(2), s2, now_secs() + 200));

        let list = DelegationCache::list();
        assert_eq!(list.len(), 2);
        assert_eq!(DelegationCache::count(), 2);
    }

    #[test]
    fn cleanup_expired_removes_old_entries() {
        DelegationCache::clear();
        let expired = pid(20);
        let valid = pid(21);

        DelegationCache::insert(expired, make_view(pid(1), expired, now_secs() - 10));
        DelegationCache::insert(valid, make_view(pid(2), valid, now_secs() + 100));

        let (before, after) = DelegationCache::cleanup_expired();
        assert_eq!(before, 2);
        assert_eq!(after, 1);

        assert!(DelegationCache::get(expired).is_none());
        assert!(DelegationCache::get(valid).is_some());
    }
}
