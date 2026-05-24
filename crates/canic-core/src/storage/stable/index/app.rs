use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    storage::{prelude::*, stable::memory::topology::APP_INDEX_ID},
};
use ic_memory::stable_structures::btreemap::BTreeMap as StableBtreeMap;
use std::cell::RefCell;

eager_static! {
    static APP_INDEX: RefCell<StableBtreeMap<CanisterRole, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(StableBtreeMap::init(crate::ic_memory_key!("canic.core.app_index.v1", AppIndex, APP_INDEX_ID)));
}

///
/// AppIndexRecord
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppIndexRecord {
    pub entries: Vec<(CanisterRole, Principal)>,
}

///
/// AppIndex
///
/// Stable-memory-backed index mapping canister roles to principals.
///
/// Invariants:
/// - Each role appears at most once.
/// - The index is authoritative; imports replace all existing entries.
/// - This structure is persisted and replicated via snapshot import/export.
///

pub struct AppIndex;

impl AppIndex {
    // cannot return an iterator because of stable memory
    #[must_use]
    pub(crate) fn export() -> AppIndexRecord {
        AppIndexRecord {
            entries: APP_INDEX.with_borrow(|map| {
                map.iter()
                    .map(|entry| (entry.key().clone(), entry.value()))
                    .collect()
            }),
        }
    }

    pub(crate) fn import(data: AppIndexRecord) {
        APP_INDEX.with_borrow_mut(|map| {
            map.clear_new();
            for (role, pid) in data.entries {
                map.insert(role, pid);
            }
        });
    }
}
