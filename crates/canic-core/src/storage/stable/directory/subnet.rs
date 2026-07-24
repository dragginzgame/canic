use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    role_contract::allocation::memory::topology::SUBNET_DIRECTORY_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

eager_static! {
    static SUBNET_DIRECTORY: RefCell<StableBtreeMap<CanisterRole, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.subnet_index.v1", ty = SubnetDirectory, id = SUBNET_DIRECTORY_ID)));
}

///
/// SubnetDirectoryData
///

/// Canonical Subnet Directory import/export snapshot.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubnetDirectoryData {
    pub entries: Vec<super::DirectoryEntryRecord>,
}

impl SubnetDirectoryData {
    pub const STATE_CONTRACT_NAME: &'static str = "SubnetDirectoryData";
}

///
/// SubnetDirectory
///
/// Stable-memory–backed model relation mapping subnet-scoped canister
/// roles to their principals.
///
/// Invariants:
/// - Each role appears at most once.
/// - This Directory is authoritative for its local projection and replaced wholesale on import.
/// - `SubnetDirectoryData` is its canonical import/export snapshot.
///

pub struct SubnetDirectory;

impl SubnetDirectory {
    // cannot return an iterator because of stable memory
    #[must_use]
    pub(crate) fn export() -> SubnetDirectoryData {
        SubnetDirectoryData {
            entries: SUBNET_DIRECTORY.with_borrow(|map| {
                map.iter()
                    .map(|entry| super::DirectoryEntryRecord {
                        role: entry.key().clone(),
                        pid: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    pub(crate) fn import(data: SubnetDirectoryData) {
        SUBNET_DIRECTORY.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.role, entry.pid);
            }
        });
    }
}
