use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    storage::{prelude::*, stable::memory::topology::SUBNET_DIRECTORY_ID},
};
use std::cell::RefCell;

eager_static! {
    static SUBNET_DIRECTORY: RefCell<BTreeMap<CanisterRole, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(SubnetDirectory, SUBNET_DIRECTORY_ID)));
}

///
/// SubnetDirectoryData
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubnetDirectoryData {
    pub entries: Vec<(CanisterRole, Principal)>,
}

///
/// SubnetDirectory
///
/// Stable-memory–backed model relation mapping subnet-scoped canister
/// roles to their principals.
///
/// Invariants:
/// - Each role appears at most once.
/// - This directory is authoritative and replaced wholesale on import.
/// - View/snapshot representations are constructed in higher layers.
///

pub struct SubnetDirectory;

impl SubnetDirectory {
    // cannot return an iterator because of stable memory
    #[must_use]
    pub(crate) fn export() -> SubnetDirectoryData {
        SubnetDirectoryData {
            entries: SUBNET_DIRECTORY.with_borrow(|map| {
                map.iter()
                    .map(|entry| (entry.key().clone(), entry.value()))
                    .collect()
            }),
        }
    }

    pub(crate) fn import(data: SubnetDirectoryData) {
        SUBNET_DIRECTORY.with_borrow_mut(|map| {
            map.clear();
            for (role, pid) in data.entries {
                map.insert(role, pid);
            }
        });
    }
}
