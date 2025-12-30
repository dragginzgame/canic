use crate::{
    cdk::{
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
        types::{Cycles, Principal},
    },
    eager_static, ic_memory,
    ids::CanisterRole,
    memory::impl_storable_unbounded,
    model::memory::id::pool::CANISTER_POOL_ID,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

//
// Stable storage
//

eager_static! {
    static CANISTER_POOL: RefCell<
        BTreeMap<Principal, CanisterPoolEntry, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(CanisterPool, CANISTER_POOL_ID)),
    );
}

///
/// CanisterPoolData
///

#[derive(Clone, Debug)]
pub struct CanisterPoolData {
    pub entries: Vec<(Principal, CanisterPoolEntry)>,
}

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
/// Composite entry stored in stable memory.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CanisterPoolEntry {
    pub header: CanisterPoolHeader,
    pub state: CanisterPoolState,
}

impl_storable_unbounded!(CanisterPoolEntry);

///
/// CanisterPoolHeader
/// Immutable, ordering-relevant metadata.
/// Set once at registration and must never change.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CanisterPoolHeader {
    pub created_at: u64,
}

///
/// CanisterPoolState
/// Mutable lifecycle state.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CanisterPoolState {
    pub cycles: Cycles,
    pub status: CanisterPoolStatus,
    pub role: Option<CanisterRole>,
    pub parent: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
}

///
/// CanisterPool
///

pub struct CanisterPool;

impl CanisterPool {
    /// Register a new canister into the pool.
    /// Sets ordering state (`created_at`) exactly once.
    pub fn register(
        pid: Principal,
        cycles: Cycles,
        status: CanisterPoolStatus,
        role: Option<CanisterRole>,
        parent: Option<Principal>,
        module_hash: Option<Vec<u8>>,
        created_at: u64,
    ) {
        let entry = CanisterPoolEntry {
            header: CanisterPoolHeader { created_at },
            state: CanisterPoolState {
                cycles,
                status,
                role,
                parent,
                module_hash,
            },
        };

        CANISTER_POOL.with_borrow_mut(|map| {
            map.insert(pid, entry);
        });
    }

    //
    // Updates (stable-structures friendly)
    //

    /// Transform the mutable state of an existing entry, preserving the immutable header.
    ///
    /// This avoids `get_mut()` (not available in stable-structures BTreeMap) and
    /// makes ordering invariants structural: callers cannot change `created_at`.
    pub(crate) fn update_state_with<F>(pid: Principal, f: F) -> bool
    where
        F: FnOnce(CanisterPoolState) -> CanisterPoolState,
    {
        CANISTER_POOL.with_borrow_mut(|map| {
            let Some(old) = map.get(&pid) else {
                return false;
            };

            let new_state = f(old.state);
            let new_entry = CanisterPoolEntry {
                header: old.header, // preserved
                state: new_state,
            };

            map.insert(pid, new_entry);
            true
        })
    }

    //
    // Queries
    //

    #[must_use]
    pub(crate) fn get(pid: Principal) -> Option<CanisterPoolEntry> {
        CANISTER_POOL.with_borrow(|map| map.get(&pid))
    }

    #[must_use]
    pub(crate) fn contains(pid: &Principal) -> bool {
        CANISTER_POOL.with_borrow(|map| map.contains_key(pid))
    }

    #[must_use]
    pub(crate) fn len() -> u64 {
        CANISTER_POOL.with_borrow(|map| map.len())
    }

    // --- Removal --------------------------------------------------------

    /// Pop the oldest READY canister from the pool.
    /// Policy: FIFO among entries whose state is `Ready`.
    #[must_use]
    pub(crate) fn pop_ready() -> Option<(Principal, CanisterPoolEntry)> {
        CANISTER_POOL.with_borrow_mut(|map| {
            let pid = map
                .iter()
                .filter(|e| e.value().state.status.is_ready())
                .min_by_key(|e| e.value().header.created_at)
                .map(|e| *e.key())?;

            map.remove(&pid).map(|entry| (pid, entry))
        })
    }

    /// Remove a specific canister from the pool, returning its entry.
    #[must_use]
    pub(crate) fn take(pid: &Principal) -> Option<CanisterPoolEntry> {
        CANISTER_POOL.with_borrow_mut(|map| map.remove(pid))
    }

    //
    // Export
    //

    #[must_use]
    pub(crate) fn export() -> CanisterPoolData {
        CanisterPoolData {
            entries: CANISTER_POOL.with_borrow(BTreeMap::to_vec),
        }
    }

    //
    // Test helpers
    //

    #[cfg(test)]
    pub(crate) fn clear() {
        CANISTER_POOL.with_borrow_mut(BTreeMap::clear);
    }

    #[cfg(test)]
    #[must_use]
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

    // --- Helpers --------------------------------------------------------

    fn pid(n: u8) -> Principal {
        Principal::self_authenticating(vec![n])
    }

    fn insert_ready_with_created_at(pid: Principal, created_at: u64, cycles: u128) {
        let entry = CanisterPoolEntry {
            header: CanisterPoolHeader { created_at },
            state: CanisterPoolState {
                cycles: cycles.into(),
                status: CanisterPoolStatus::Ready,
                role: None,
                parent: None,
                module_hash: None,
            },
        };

        CANISTER_POOL.with_borrow_mut(|map| {
            map.insert(pid, entry);
        });
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
            1,
        );
        CanisterPool::register(
            p2,
            200u128.into(),
            CanisterPoolStatus::Ready,
            None,
            None,
            None,
            2,
        );

        let data = CanisterPool::export();
        assert_eq!(data.entries.len(), 2);

        let (_, e1) = data.entries.iter().find(|(id, _)| *id == p1).unwrap();
        let (_, e2) = data.entries.iter().find(|(id, _)| *id == p2).unwrap();

        assert_eq!(e1.state.cycles, 100u128.into());
        assert_eq!(e2.state.cycles, 200u128.into());

        // header must exist and be non-zero
        assert!(e1.header.created_at > 0);
        assert!(e2.header.created_at > 0);
    }

    #[test]
    fn pop_ready_returns_oldest_by_created_at() {
        CanisterPool::clear();

        let p1 = pid(1);
        let p2 = pid(2);

        insert_ready_with_created_at(p1, 1, 1);
        insert_ready_with_created_at(p2, 2, 2);

        let (pid, entry) = CanisterPool::pop_ready().expect("expected ready entry");
        assert_eq!(pid, p1);
        assert_eq!(entry.state.cycles, 1u128.into());
    }

    #[test]
    fn update_state_preserves_header() {
        CanisterPool::clear();

        let p = pid(1);

        CanisterPool::register(
            p,
            10u128.into(),
            CanisterPoolStatus::PendingReset,
            None,
            None,
            None,
            42,
        );

        let before = CanisterPool::get(p).unwrap().header.created_at;

        let updated = CanisterPool::update_state_with(p, |mut state| {
            state.cycles = 99u128.into();
            state.status = CanisterPoolStatus::Ready;
            state
        });

        assert!(updated);

        let entry = CanisterPool::get(p).unwrap();
        assert_eq!(entry.state.cycles, 99u128.into());
        assert!(entry.state.status.is_ready());
        assert_eq!(entry.header.created_at, before);
    }

    #[test]
    fn update_state_returns_false_for_missing_pid() {
        CanisterPool::clear();

        let updated = CanisterPool::update_state_with(pid(9), |state| state);
        assert!(!updated);
    }

    #[test]
    fn take_removes_specific_entry() {
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
            1,
        );
        CanisterPool::register(
            p2,
            456u128.into(),
            CanisterPoolStatus::Ready,
            None,
            None,
            None,
            2,
        );

        let removed = CanisterPool::take(&p1).unwrap();
        assert_eq!(removed.state.cycles, 123u128.into());

        let remaining = CanisterPool::export();
        assert_eq!(remaining.entries.len(), 1);
        assert_eq!(remaining.entries[0].0, p2);
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
            1,
        );
        assert!(!CanisterPool::is_empty());

        CanisterPool::clear();
        assert!(CanisterPool::is_empty());
    }
}
