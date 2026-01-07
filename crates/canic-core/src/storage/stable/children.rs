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
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    storage::{
        canister::CanisterSummary, prelude::*, stable::memory::children::CANISTER_CHILDREN_ID,
    },
};
use std::cell::RefCell;

eager_static! {
    //
    // CANISTER_CHILDREN
    //
    static CANISTER_CHILDREN: RefCell<
        BTreeMap<Principal, CanisterSummary, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(CanisterChildren, CANISTER_CHILDREN_ID)),
    );
}

///
/// CanisterChildrenData
///

#[derive(Clone, Debug)]
pub struct CanisterChildrenData {
    pub entries: Vec<(Principal, CanisterSummary)>,
}

///
/// CanisterChildren
///

pub struct CanisterChildren;

impl CanisterChildren {
    #[must_use]
    pub fn export() -> CanisterChildrenData {
        CanisterChildrenData {
            entries: CANISTER_CHILDREN
                .with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect()),
        }
    }

    pub(crate) fn import(data: CanisterChildrenData) {
        CANISTER_CHILDREN.with_borrow_mut(|map| {
            map.clear();
            for (pid, entry) in data.entries {
                map.insert(pid, entry);
            }
        });
    }
}
