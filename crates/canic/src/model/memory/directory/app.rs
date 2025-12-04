use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    model::memory::{
        directory::{DirectoryView, PrincipalList},
        id::directory::APP_DIRECTORY_ID,
    },
    types::CanisterType,
};
use std::cell::RefCell;

//
// APP_DIRECTORY
//

eager_static! {
    static APP_DIRECTORY: RefCell<BTreeMap<CanisterType, PrincipalList, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(AppDirectory, APP_DIRECTORY_ID)));
}

///
/// AppDirectory
///

pub(crate) struct AppDirectory;

impl AppDirectory {
    #[must_use]
    #[expect(dead_code)]
    pub fn get(ty: &CanisterType) -> Option<PrincipalList> {
        APP_DIRECTORY.with_borrow(|map| map.get(ty))
    }

    // cannot return an iterator because of stable memory
    #[must_use]
    pub fn view() -> DirectoryView {
        APP_DIRECTORY.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }

    pub fn import(view: DirectoryView) {
        APP_DIRECTORY.with_borrow_mut(|map| {
            map.clear();
            for (ty, pids) in view {
                map.insert(ty, pids);
            }
        });
    }
}
