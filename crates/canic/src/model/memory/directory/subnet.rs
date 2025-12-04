use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    model::memory::{
        directory::{DirectoryView, PrincipalList},
        id::directory::SUBNET_DIRECTORY_ID,
    },
    types::CanisterType,
};
use std::cell::RefCell;

//
// SUBNET_DIRECTORY
//

eager_static! {
    static SUBNET_DIRECTORY: RefCell<BTreeMap<CanisterType, PrincipalList, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(SubnetDirectory, SUBNET_DIRECTORY_ID)));
}

///
/// SubnetDirectory
///

pub(crate) struct SubnetDirectory;

impl SubnetDirectory {
    #[must_use]
    #[expect(dead_code)]
    pub fn get(ty: &CanisterType) -> Option<PrincipalList> {
        SUBNET_DIRECTORY.with_borrow(|map| map.get(ty))
    }

    // cannot return an iterator because of stable memory
    #[must_use]
    pub fn view() -> DirectoryView {
        SUBNET_DIRECTORY.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }

    pub fn import(view: DirectoryView) {
        SUBNET_DIRECTORY.with_borrow_mut(|map| {
            map.clear();
            for (ty, pids) in view {
                map.insert(ty, pids);
            }
        });
    }
}
