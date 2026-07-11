use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    role_contract::allocation::memory::topology::SUBNET_INDEX_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

eager_static! {
    static SUBNET_INDEX: RefCell<StableBtreeMap<CanisterRole, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.subnet_index.v1", ty = SubnetIndex, id = SUBNET_INDEX_ID)));
}

///
/// SubnetIndexData
///

/// Canonical subnet-index import/export snapshot.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubnetIndexData {
    pub entries: Vec<super::IndexEntryRecord>,
}

impl SubnetIndexData {
    pub const STATE_CONTRACT_NAME: &'static str = "SubnetIndexData";
}

///
/// SubnetIndex
///
/// Stable-memory–backed model relation mapping subnet-scoped canister
/// roles to their principals.
///
/// Invariants:
/// - Each role appears at most once.
/// - This index is authoritative and replaced wholesale on import.
/// - `SubnetIndexData` is its canonical import/export snapshot.
///

pub struct SubnetIndex;

impl SubnetIndex {
    // cannot return an iterator because of stable memory
    #[must_use]
    pub(crate) fn export() -> SubnetIndexData {
        SubnetIndexData {
            entries: SUBNET_INDEX.with_borrow(|map| {
                map.iter()
                    .map(|entry| super::IndexEntryRecord {
                        role: entry.key().clone(),
                        pid: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    pub(crate) fn import(data: SubnetIndexData) {
        SUBNET_INDEX.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.role, entry.pid);
            }
        });
    }
}
