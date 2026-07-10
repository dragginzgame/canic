//! CanisterChildren
//!
//! Stable-memory–backed projection of direct child canisters for the
//! current canister.
//!
//! This is not an authoritative registry. Canonical child derivation lives in
//! `SubnetRegistry::children` / `SubnetRegistryOps::children`; entries here are
//! populated via topology snapshot import during cascade workflows and represent
//! a cached projection of the global subnet registry.
//!
//! The contents are replaced wholesale on import.

use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    role_contract::allocation::memory::topology::CANISTER_CHILDREN_ID,
    storage::{canister::CanisterRecord, prelude::*},
};
use ic_memory::stable_structures::btreemap::BTreeMap as StableBtreeMap;
use std::cell::RefCell;

eager_static! {
    //
    // CANISTER_CHILDREN
    //
    static CANISTER_CHILDREN: RefCell<
        StableBtreeMap<Principal, CanisterRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!("canic.core.canister_children.v1", CanisterChildren, CANISTER_CHILDREN_ID)),
    );
}

///
/// CanisterChildrenRecord
///

#[derive(Clone, Debug)]
pub struct CanisterChildrenRecord {
    pub entries: Vec<(Principal, CanisterRecord)>,
}

///
/// CanisterChildren
///

pub struct CanisterChildren;

impl CanisterChildren {
    #[must_use]
    pub(crate) fn get(pid: Principal) -> Option<CanisterRecord> {
        CANISTER_CHILDREN.with_borrow(|map| map.get(&pid))
    }

    #[must_use]
    pub fn export() -> CanisterChildrenRecord {
        CanisterChildrenRecord {
            entries: CANISTER_CHILDREN
                .with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect()),
        }
    }

    pub(crate) fn import(data: CanisterChildrenRecord) {
        CANISTER_CHILDREN.with_borrow_mut(|map| {
            map.clear_new();
            for (pid, entry) in data.entries {
                map.insert(pid, entry);
            }
        });
    }
}
