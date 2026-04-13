use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    storage::{prelude::*, stable::memory::topology::APP_INDEX_ID},
};
use std::cell::RefCell;

eager_static! {
    static APP_INDEX: RefCell<BTreeMap<CanisterRole, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(AppIndex, APP_INDEX_ID)));
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
            map.clear();
            for (role, pid) in data.entries {
                map.insert(role, pid);
            }
        });
    }
}
