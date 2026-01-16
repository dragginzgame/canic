use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    storage::{prelude::*, stable::memory::topology::APP_DIRECTORY_ID},
};
use std::cell::RefCell;

eager_static! {
    static APP_DIRECTORY: RefCell<BTreeMap<CanisterRole, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(AppDirectory, APP_DIRECTORY_ID)));
}

///
/// AppDirectoryData
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppDirectoryData {
    pub entries: Vec<(CanisterRole, Principal)>,
}

///
/// AppDirectory
///
/// Stable-memory–backed directory mapping canister roles to principals.
///
/// Invariants:
/// - Each role appears at most once.
/// - The directory is authoritative; imports replace all existing entries.
/// - This structure is persisted and replicated via snapshot import/export.
///

pub struct AppDirectory;

impl AppDirectory {
    // cannot return an iterator because of stable memory
    #[must_use]
    pub(crate) fn export() -> AppDirectoryData {
        AppDirectoryData {
            entries: APP_DIRECTORY.with_borrow(|map| {
                map.iter()
                    .map(|entry| (entry.key().clone(), entry.value()))
                    .collect()
            }),
        }
    }

    pub(crate) fn import(data: AppDirectoryData) {
        APP_DIRECTORY.with_borrow_mut(|map| {
            map.clear();
            for (role, pid) in data.entries {
                map.insert(role, pid);
            }
        });
    }
}
