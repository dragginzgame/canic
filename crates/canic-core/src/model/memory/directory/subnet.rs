use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    ids::CanisterRole,
    model::memory::id::directory::SUBNET_DIRECTORY_ID,
};
use candid::Principal;
use std::cell::RefCell;

eager_static! {
    static SUBNET_DIRECTORY: RefCell<BTreeMap<CanisterRole, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(SubnetDirectory, SUBNET_DIRECTORY_ID)));
}

///
/// SubnetDirectoryData
///

pub type SubnetDirectoryData = Vec<(CanisterRole, Principal)>;

///
/// SubnetDirectory
///

pub struct SubnetDirectory;

impl SubnetDirectory {
    // cannot return an iterator because of stable memory
    #[must_use]
    pub(crate) fn export() -> SubnetDirectoryData {
        SUBNET_DIRECTORY.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }

    pub(crate) fn import(data: SubnetDirectoryData) {
        SUBNET_DIRECTORY.with_borrow_mut(|map| {
            map.clear();
            for (role, pid) in data {
                map.insert(role, pid);
            }
        });
    }
}
