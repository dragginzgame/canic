use crate::{
    cdk::{
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
        types::{Cycles, Principal},
        utils::time::now_secs,
    },
    eager_static, ic_memory,
    ids::CanisterRole,
    memory::impl_storable_unbounded,
    model::memory::id::pool::CANISTER_POOL_ID,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

eager_static! {
    static CANISTER_POOL: RefCell<BTreeMap<Principal, CanisterPoolEntry, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(
            ic_memory!(CanisterPool, CANISTER_POOL_ID),
        ));
}

///
/// CanisterPoolData
///

pub type CanisterPoolData = Vec<(Principal, CanisterPoolEntry)>;

///
/// CanisterPoolStatus
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolStatus {
    PendingReset,
    #[default]
    Ready,
    Failed {
        reason: String,
    },
}

impl CanisterPoolStatus {
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

///
/// CanisterPoolEntry
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CanisterPoolEntry {
    pub created_at: u64,
    pub cycles: Cycles,
    #[serde(default)]
    pub status: CanisterPoolStatus,
    #[serde(default)]
    pub role: Option<CanisterRole>,
    #[serde(default)]
    pub parent: Option<Principal>,
    #[serde(default)]
    pub module_hash: Option<Vec<u8>>,
}

impl_storable_unbounded!(CanisterPoolEntry);

///
/// CanisterPool
///

pub struct CanisterPool;

impl CanisterPool {
    /// Register a canister into the pool.
    pub fn register(
        pid: Principal,
        cycles: Cycles,
        status: CanisterPoolStatus,
        role: Option<CanisterRole>,
        parent: Option<Principal>,
        module_hash: Option<Vec<u8>>,
    ) {
        let entry = CanisterPoolEntry {
            created_at: now_secs(),
            cycles,
            status,
            role,
            parent,
            module_hash,
        };

        CANISTER_POOL.with_borrow_mut(|map| {
            map.insert(pid, entry);
        });
    }

    #[must_use]
    pub(crate) fn get(pid: Principal) -> Option<CanisterPoolEntry> {
        CANISTER_POOL.with_borrow(|map| map.get(&pid))
    }

    #[must_use]
    pub(crate) fn update(pid: Principal, entry: CanisterPoolEntry) -> bool {
        CANISTER_POOL.with_borrow_mut(|map| {
            if map.contains_key(&pid) {
                map.insert(pid, entry);
                true
            } else {
                false
            }
        })
    }

    /// Pop the oldest ready canister from the pool.
    #[must_use]
    pub(crate) fn pop_ready() -> Option<(Principal, CanisterPoolEntry)> {
        CANISTER_POOL.with_borrow_mut(|map| {
            let min_pid = map
                .iter()
                .filter(|entry| entry.value().status.is_ready())
                .min_by_key(|entry| entry.value().created_at)
                .map(|entry| *entry.key())?;
            map.remove(&min_pid).map(|entry| (min_pid, entry))
        })
    }

    /// Return true if the pool contains the given canister.
    #[must_use]
    pub(crate) fn contains(pid: &Principal) -> bool {
        CANISTER_POOL.with_borrow(|map| map.contains_key(pid))
    }

    /// Remove a specific canister from the pool, returning its entry.
    #[must_use]
    pub(crate) fn take(pid: &Principal) -> Option<CanisterPoolEntry> {
        CANISTER_POOL.with_borrow_mut(|map| map.remove(pid))
    }

    /// Remove a specific canister from the pool.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn remove(pid: &Principal) -> Option<CanisterPoolEntry> {
        CANISTER_POOL.with_borrow_mut(|map| map.remove(pid))
    }

    /// Export the pool as a vector of (Principal, Entry).
    #[must_use]
    pub(crate) fn export() -> CanisterPoolData {
        CANISTER_POOL.with_borrow(BTreeMap::to_vec)
    }

    /// Clear the pool (mainly for tests).
    #[cfg(test)]
    pub(crate) fn clear() {
        CANISTER_POOL.with_borrow_mut(BTreeMap::clear);
    }

    /// Return the current pool size.
    #[must_use]
    pub(crate) fn len() -> u64 {
        CANISTER_POOL.with_borrow(|map| map.len())
    }

    /// Return whether the pool is empty.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn is_empty() -> bool {
        CANISTER_POOL.with_borrow(|map| map.is_empty())
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    fn pid(n: u8) -> Principal {
        Principal::self_authenticating(vec![n])
    }

    #[test]
    fn register_and_export() {
        CanisterPool::clear();

        let p1 = pid(1);
        let p2 = pid(2);

        CanisterPool::register(
            p1,
            100u128.into(),
            CanisterPoolStatus::Ready,
            None,
            None,
            None,
        );
        CanisterPool::register(
            p2,
            200u128.into(),
            CanisterPoolStatus::Ready,
            None,
            None,
            None,
        );

        let data = CanisterPool::export();
        assert_eq!(data.len(), 2);

        let entry1 = data.iter().find(|(id, _)| *id == p1).unwrap();
        assert_eq!(entry1.1.cycles, 100u128.into());

        let entry2 = data.iter().find(|(id, _)| *id == p2).unwrap();
        assert_eq!(entry2.1.cycles, 200u128.into());
    }

    #[test]
    fn remove_specific_pid() {
        CanisterPool::clear();

        let p1 = pid(1);
        let p2 = pid(2);

        CanisterPool::register(
            p1,
            123u128.into(),
            CanisterPoolStatus::Ready,
            None,
            None,
            None,
        );
        CanisterPool::register(
            p2,
            456u128.into(),
            CanisterPoolStatus::Ready,
            None,
            None,
            None,
        );

        let removed = CanisterPool::remove(&p1).unwrap();
        assert_eq!(removed.cycles, 123u128.into());

        // only p2 should remain
        let data = CanisterPool::export();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].0, p2);
    }

    #[test]
    fn clear_resets_pool() {
        CanisterPool::clear();

        CanisterPool::register(
            pid(1),
            10u128.into(),
            CanisterPoolStatus::Ready,
            None,
            None,
            None,
        );
        assert!(!CanisterPool::is_empty());

        CanisterPool::clear();
        assert!(CanisterPool::is_empty());
    }
}
