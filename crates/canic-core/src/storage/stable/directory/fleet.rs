use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    role_contract::allocation::memory::topology::FLEET_DIRECTORY_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

eager_static! {
    static FLEET_DIRECTORY: RefCell<StableBtreeMap<CanisterRole, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.app_index.v1", ty = FleetDirectory, id = FLEET_DIRECTORY_ID)));
}

///
/// FleetDirectoryData
///

/// Canonical Fleet Directory import/export snapshot.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetDirectoryData {
    pub entries: Vec<super::DirectoryEntryRecord>,
}

impl FleetDirectoryData {
    pub const STATE_CONTRACT_NAME: &'static str = "FleetDirectoryData";
}

///
/// FleetDirectory
///
/// Stable-memory-backed Fleet Directory mapping canister roles to principals.
///
/// Invariants:
/// - Each role appears at most once.
/// - The Directory is authoritative for its local projection; imports replace all entries.
/// - This structure is persisted and replicated through `FleetDirectoryData`.
///

pub struct FleetDirectory;

impl FleetDirectory {
    // cannot return an iterator because of stable memory
    #[must_use]
    pub(crate) fn export() -> FleetDirectoryData {
        FleetDirectoryData {
            entries: FLEET_DIRECTORY.with_borrow(|map| {
                map.iter()
                    .map(|entry| super::DirectoryEntryRecord {
                        role: entry.key().clone(),
                        pid: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    pub(crate) fn import(data: FleetDirectoryData) {
        FLEET_DIRECTORY.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.role, entry.pid);
            }
        });
    }
}
