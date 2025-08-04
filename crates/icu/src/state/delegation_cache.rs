use crate::{Log, log, state::DelegationSessionInfo, utils::time::now_secs};
use candid::Principal;
use std::{cell::RefCell, collections::HashMap};

//
// DELEGATION_CACHE
//

thread_local! {
    static DELEGATION_CACHE: RefCell<HashMap<Principal, DelegationSessionInfo>> = RefCell::new(HashMap::new());
}

///
/// DelegationCache
///

pub struct DelegationCache {}

impl DelegationCache {
    #[must_use]
    pub fn get(session_pid: Principal) -> Option<DelegationSessionInfo> {
        DELEGATION_CACHE.with_borrow(|map| map.get(&session_pid).cloned())
    }

    pub fn insert(session_pid: Principal, session: DelegationSessionInfo) {
        DELEGATION_CACHE.with_borrow_mut(|map| {
            map.insert(session_pid, session);
        });
    }

    #[must_use]
    pub fn remove(session_pid: Principal) -> bool {
        DELEGATION_CACHE.with_borrow_mut(|map| map.remove(&session_pid).is_some())
    }

    #[must_use]
    pub fn list() -> Vec<(Principal, DelegationSessionInfo)> {
        DELEGATION_CACHE.with_borrow(|map| map.iter().map(|(k, v)| (*k, v.clone())).collect())
    }

    pub fn cleanup_expired() {
        let now = now_secs();

        let before = DELEGATION_CACHE.with_borrow(HashMap::len);
        DELEGATION_CACHE.with_borrow_mut(|map| {
            map.retain(|_, s| s.expires_at.is_some_and(|ts| ts > now));
        });
        let after = DELEGATION_CACHE.with_borrow(HashMap::len);

        log!(
            Log::Info,
            "cleaned up sessions, before: {before}, after: {after}"
        );
    }

    #[must_use]
    pub fn count() -> usize {
        DELEGATION_CACHE.with_borrow(HashMap::len)
    }
}
