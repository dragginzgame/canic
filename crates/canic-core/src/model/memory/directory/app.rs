use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    ids::CanisterRole,
    model::memory::id::directory::APP_DIRECTORY_ID,
};
use candid::Principal;
use std::cell::RefCell;

//
// APP_DIRECTORY
//

eager_static! {
    static APP_DIRECTORY: RefCell<BTreeMap<CanisterRole, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(AppDirectory, APP_DIRECTORY_ID)));
}

///
/// AppDirectory
///

pub struct AppDirectory;

impl AppDirectory {
    // cannot return an iterator because of stable memory
    #[must_use]
    pub(crate) fn export() -> Vec<(CanisterRole, Principal)> {
        APP_DIRECTORY.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }

    pub(crate) fn import(data: Vec<(CanisterRole, Principal)>) {
        APP_DIRECTORY.with_borrow_mut(|map| {
            map.clear();
            for (role, pid) in data {
                map.insert(role, pid);
            }
        });
    }
}
