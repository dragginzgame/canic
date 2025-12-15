use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    ids::CanisterRole,
    model::memory::{
        directory::{DirectoryView, PrincipalList},
        id::directory::APP_DIRECTORY_ID,
    },
};
use std::cell::RefCell;

//
// APP_DIRECTORY
//

eager_static! {
    static APP_DIRECTORY: RefCell<BTreeMap<CanisterRole, PrincipalList, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(AppDirectory, APP_DIRECTORY_ID)));
}

///
/// AppDirectory
///

pub struct AppDirectory;

impl AppDirectory {
    #[must_use]
    #[expect(dead_code)]
    pub(crate) fn get(role: &CanisterRole) -> Option<PrincipalList> {
        APP_DIRECTORY.with_borrow(|map| map.get(role))
    }

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
            for (ty, pids) in view {
                map.insert(ty, pids);
            }
        });
    }
}
