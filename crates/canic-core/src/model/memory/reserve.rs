use crate::{
    cdk::{
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
        types::Principal,
        utils::time::now_secs,
    },
    eager_static, ic_memory,
    ids::CanisterRole,
    memory::impl_storable_unbounded,
    model::memory::id::root::CANISTER_RESERVE_ID,
    types::Cycles,
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

//
// CANISTER_RESERVE
//

eager_static! {
    static CANISTER_RESERVE: RefCell<BTreeMap<Principal, CanisterReserveEntry, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(
            ic_memory!(CanisterReserve, CANISTER_RESERVE_ID),
        ));
}

///
/// CanisterReserveView
///

pub type CanisterReserveView = Vec<(Principal, CanisterReserveEntry)>;

///
/// CanisterReserveEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterReserveEntry {
    pub created_at: u64,
    pub cycles: Cycles,
    #[serde(default)]
    pub role: Option<CanisterRole>,
    #[serde(default)]
    pub parent: Option<Principal>,
    #[serde(default)]
    pub module_hash: Option<Vec<u8>>,
}

impl_storable_unbounded!(CanisterReserveEntry);

///
/// CanisterReserve
///

pub(crate) struct CanisterReserve;

impl CanisterReserve {
    /// Register a canister into the reserve.
    pub(crate) fn register(
        pid: Principal,
        cycles: Cycles,
        role: Option<CanisterRole>,
        parent: Option<Principal>,
        module_hash: Option<Vec<u8>>,
    ) {
        let entry = CanisterReserveEntry {
            created_at: now_secs(),
            cycles,
            role,
            parent,
            module_hash,
        };

        CANISTER_RESERVE.with_borrow_mut(|map| {
            map.insert(pid, entry);
        });
    }

    /// Pop the oldest canister from the reserve.
    #[must_use]
    pub(crate) fn pop_first() -> Option<(Principal, CanisterReserveEntry)> {
        CANISTER_RESERVE.with_borrow_mut(|map| {
            let min_pid = map
                .iter()
                .min_by_key(|entry| entry.value().created_at)
                .map(|entry| *entry.key())?;
            map.remove(&min_pid).map(|entry| (min_pid, entry))
        })
    }

    /// Return true if the reserve contains the given canister.
    #[must_use]
    pub(crate) fn contains(pid: &Principal) -> bool {
        CANISTER_RESERVE.with_borrow(|map| map.contains_key(pid))
    }

    /// Remove a specific canister from the reserve, returning its entry.
    #[must_use]
    pub(crate) fn take(pid: &Principal) -> Option<CanisterReserveEntry> {
        CANISTER_RESERVE.with_borrow_mut(|map| map.remove(pid))
    }

    /// Remove a specific canister from the reserve.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn remove(pid: &Principal) -> Option<CanisterReserveEntry> {
        CANISTER_RESERVE.with_borrow_mut(|map| map.remove(pid))
    }

    /// Export the reserve as a vector of (Principal, Entry).
    #[must_use]
    pub(crate) fn export() -> CanisterReserveView {
        CANISTER_RESERVE.with_borrow(BTreeMap::to_vec)
    }

    /// Clear the reserve (mainly for tests).
    #[cfg(test)]
    pub(crate) fn clear() {
        CANISTER_RESERVE.with_borrow_mut(BTreeMap::clear);
    }

    /// Return the current reserve size.
    #[must_use]
    pub(crate) fn len() -> u64 {
        CANISTER_RESERVE.with_borrow(|map| map.len())
    }

    /// Return whether the reserve is empty.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn is_empty() -> bool {
        CANISTER_RESERVE.with_borrow(|map| map.is_empty())
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
        CanisterReserve::clear();

        let p1 = pid(1);
        let p2 = pid(2);

        CanisterReserve::register(p1, 100u128.into(), None, None, None);
        CanisterReserve::register(p2, 200u128.into(), None, None, None);

        let view = CanisterReserve::export();
        assert_eq!(view.len(), 2);

        let entry1 = view.iter().find(|(id, _)| *id == p1).unwrap();
        assert_eq!(entry1.1.cycles, 100u128.into());

        let entry2 = view.iter().find(|(id, _)| *id == p2).unwrap();
        assert_eq!(entry2.1.cycles, 200u128.into());
    }

    #[test]
    fn remove_specific_pid() {
        CanisterReserve::clear();

        let p1 = pid(1);
        let p2 = pid(2);

        CanisterReserve::register(p1, 123u128.into(), None, None, None);
        CanisterReserve::register(p2, 456u128.into(), None, None, None);

        let removed = CanisterReserve::remove(&p1).unwrap();
        assert_eq!(removed.cycles, 123u128.into());

        // only p2 should remain
        let view = CanisterReserve::export();
        assert_eq!(view.len(), 1);
        assert_eq!(view[0].0, p2);
    }

    #[test]
    fn clear_resets_reserve() {
        CanisterReserve::clear();

        CanisterReserve::register(pid(1), 10u128.into(), None, None, None);
        assert!(!CanisterReserve::is_empty());

        CanisterReserve::clear();
        assert!(CanisterReserve::is_empty());
    }
}
