use crate::{Log, log, state::DelegationSessionView, utils::time::now_secs};
use candid::Principal;
use std::{cell::RefCell, collections::HashMap};

//
// DELEGATION_CACHE
//

thread_local! {
    static DELEGATION_CACHE: RefCell<HashMap<Principal, DelegationSessionView>> = RefCell::new(HashMap::new());
}

///
/// DelegationCache
///

pub struct DelegationCache {}

impl DelegationCache {
    //
    // INTERNAL ACCESSORS
    //

    pub fn with<R>(f: impl FnOnce(&HashMap<Principal, DelegationSessionView>) -> R) -> R {
        DELEGATION_CACHE.with_borrow(|cell| f(cell))
    }

    pub fn with_mut<R>(f: impl FnOnce(&mut HashMap<Principal, DelegationSessionView>) -> R) -> R {
        DELEGATION_CACHE.with_borrow_mut(|cell| f(cell))
    }

    //
    // METHODS
    //

    #[must_use]
    pub fn is_empty() -> bool {
        Self::with(HashMap::is_empty)
    }

    #[must_use]
    pub fn get(session_pid: Principal) -> Option<DelegationSessionView> {
        Self::with(|map| map.get(&session_pid).cloned())
    }

    pub fn insert(session_pid: Principal, session: DelegationSessionView) {
        Self::with_mut(|map| {
            map.insert(session_pid, session);
        });
    }

    #[must_use]
    pub fn remove(session_pid: Principal) -> bool {
        Self::with_mut(|map| map.remove(&session_pid).is_some())
    }

    #[must_use]
    pub fn list() -> Vec<(Principal, DelegationSessionView)> {
        Self::with(|map| map.iter().map(|(k, v)| (*k, v.clone())).collect())
    }

    pub fn cleanup_expired() {
        let now = now_secs();

        let before = Self::with(HashMap::len);
        Self::with_mut(|map| map.retain(|_, s| s.expires_at > now));
        let after = Self::with(HashMap::len);

        log!(
            Log::Info,
            "cleaned up sessions, before: {before}, after: {after}"
        );
    }

    #[must_use]
    pub fn count() -> usize {
        Self::with(HashMap::len)
    }
}
