use crate::{
    Error,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    memory::{
        directory::{DirectoryError, DirectoryView, PrincipalList},
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

pub struct SubnetDirectory;

impl SubnetDirectory {
    #[must_use]
    pub fn get(ty: &CanisterType) -> Option<PrincipalList> {
        SUBNET_DIRECTORY.with_borrow(|map| map.get(ty))
    }

    pub fn try_get(ty: &CanisterType) -> Result<PrincipalList, Error> {
        Self::get(ty).ok_or_else(|| DirectoryError::TypeNotFound(ty.clone()).into())
    }

    //
    // Import & Export
    //

    pub fn import(view: DirectoryView) {
        SUBNET_DIRECTORY.with_borrow_mut(|map| {
            map.clear();
            for (ty, pids) in view {
                map.insert(ty, pids);
            }
        });
    }

    #[must_use]
    pub fn export() -> DirectoryView {
        SUBNET_DIRECTORY.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }
}
