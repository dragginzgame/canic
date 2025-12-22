use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    ids::CanisterRole,
    model::memory::{
        directory::{DirectoryView, PrincipalList},
        id::directory::SUBNET_DIRECTORY_ID,
    },
};
use std::cell::RefCell;

//
// SUBNET_DIRECTORY
//

eager_static! {
    static SUBNET_DIRECTORY: RefCell<BTreeMap<CanisterRole, PrincipalList, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(SubnetDirectory, SUBNET_DIRECTORY_ID)));
}

///
/// SubnetDirectory
///

pub struct SubnetDirectory;

impl SubnetDirectory {
    #[must_use]
    #[expect(dead_code)]
    pub(crate) fn get(role: &CanisterRole) -> Option<PrincipalList> {
        SUBNET_DIRECTORY.with_borrow(|map| map.get(role))
    }

    // cannot return an iterator because of stable memory
    #[must_use]
    pub(crate) fn view() -> DirectoryView {
        SUBNET_DIRECTORY.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }

    pub(crate) fn import(view: DirectoryView) {
        SUBNET_DIRECTORY.with_borrow_mut(|map| {
            map.clear();
            for (role, pids) in view {
                map.insert(role, pids);
            }
        });
    }
}
