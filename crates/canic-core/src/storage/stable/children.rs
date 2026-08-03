//! CanisterChildren
//!
//! Stable-memory–backed projection of direct child canisters for the
//! current canister.
//!
//! This is not a subnet-wide Registry. It is the per-canister direct-child
//! evidence consumed by local topology and placement workflows. Entries are
//! populated only through validated topology cascade application. Fleet Subnet
//! Root Component membership remains a separate control-plane authority.
//!
//! The contents are replaced wholesale on import.

use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    role_contract::allocation::memory::runtime::RUNTIME_CANISTER_CHILDREN_ID,
    storage::{
        canister::{CanisterEntryRecord, CanisterRecord},
        prelude::*,
    },
};
use std::cell::RefCell;

eager_static! {
    //
    // CANISTER_CHILDREN
    //
    static CANISTER_CHILDREN: RefCell<
        StableBtreeMap<Principal, CanisterRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.runtime.canister_children.v1", ty = CanisterChildren, id = RUNTIME_CANISTER_CHILDREN_ID)),
    );
}

///
/// CanisterChildrenData
///
/// Canonical direct-child projection snapshot.
///

#[derive(Clone, Debug)]
pub struct CanisterChildrenData {
    pub entries: Vec<CanisterEntryRecord>,
}

impl CanisterChildrenData {
    pub const STATE_CONTRACT_NAME: &'static str = "CanisterChildrenData";
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
    pub fn export() -> CanisterChildrenData {
        CanisterChildrenData {
            entries: CANISTER_CHILDREN.with_borrow(|map| {
                map.iter()
                    .map(|entry| CanisterEntryRecord {
                        pid: *entry.key(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    pub(crate) fn import(data: CanisterChildrenData) {
        CANISTER_CHILDREN.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.pid, entry.record);
            }
        });
    }
}
