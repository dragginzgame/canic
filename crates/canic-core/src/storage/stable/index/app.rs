use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    role_contract::allocation::memory::topology::APP_INDEX_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

eager_static! {
    static APP_INDEX: RefCell<StableBtreeMap<CanisterRole, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.app_index.v1", ty = AppIndex, id = APP_INDEX_ID)));
}

///
/// AppIndexData
///

/// Canonical app-index import/export snapshot.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppIndexData {
    pub entries: Vec<super::IndexEntryRecord>,
}

impl AppIndexData {
    pub const STATE_CONTRACT_NAME: &'static str = "AppIndexData";
}

///
/// AppIndex
///
/// Stable-memory-backed index mapping canister roles to principals.
///
/// Invariants:
/// - Each role appears at most once.
/// - The index is authoritative; imports replace all existing entries.
/// - This structure is persisted and replicated through `AppIndexData`.
///

pub struct AppIndex;

impl AppIndex {
    // cannot return an iterator because of stable memory
    #[must_use]
    pub(crate) fn export() -> AppIndexData {
        AppIndexData {
            entries: APP_INDEX.with_borrow(|map| {
                map.iter()
                    .map(|entry| super::IndexEntryRecord {
                        role: entry.key().clone(),
                        pid: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    pub(crate) fn import(data: AppIndexData) {
        APP_INDEX.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.role, entry.pid);
            }
        });
    }
}
