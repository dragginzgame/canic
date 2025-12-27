use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    dto::directory::DirectoryView,
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
    pub(crate) fn view() -> DirectoryView {
        APP_DIRECTORY.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }

    pub(crate) fn import(view: DirectoryView) {
        APP_DIRECTORY.with_borrow_mut(|map| {
            map.clear();
            for (role, pid) in view {
                map.insert(role, pid);
            }
        });
    }
}
