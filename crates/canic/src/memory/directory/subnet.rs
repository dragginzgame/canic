use crate::{
    Error, ThisError,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    memory::{MemoryError, directory::DirectoryError, id::directory::SUBNET_DIRECTORY_ID},
    types::{CanisterType, Principal},
};
use std::cell::RefCell;

//
// SUBNET_DIRECTORY
//

eager_static! {
    static SUBNET_DIRECTORY: RefCell<BTreeMap<CanisterType, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(SubnetDirectory, SUBNET_DIRECTORY_ID)));
}

///
/// SubnetDirectoryError
///

#[derive(Debug, ThisError)]
pub enum SubnetDirectoryError {
    #[error("canister already in subnet directory: {0}")]
    AlreadyRegistered(CanisterType),

    #[error("canister not in subnet directory: {0}")]
    TypeNotFound(CanisterType),
}

impl From<SubnetDirectoryError> for Error {
    fn from(err: SubnetDirectoryError) -> Self {
        MemoryError::from(DirectoryError::from(err)).into()
    }
}

///
/// SubnetDirectory
///

pub struct SubnetDirectory;

pub type SubnetDirectoryView = Vec<(CanisterType, Principal)>;

impl SubnetDirectory {
    #[must_use]
    pub fn get(ty: &CanisterType) -> Option<Principal> {
        SUBNET_DIRECTORY.with_borrow(|map| map.get(ty))
    }

    pub fn try_get(ty: &CanisterType) -> Result<Principal, Error> {
        Self::get(ty).ok_or_else(|| SubnetDirectoryError::TypeNotFound(ty.clone()).into())
    }

    pub fn register(ty: &CanisterType, pid: Principal) -> Result<(), Error> {
        SUBNET_DIRECTORY.with_borrow_mut(|map| {
            if map.contains_key(ty) {
                return Err(SubnetDirectoryError::AlreadyRegistered(ty.clone()).into());
            }
            map.insert(ty.clone(), pid);

            Ok(())
        })
    }

    #[must_use]
    pub fn remove(ty: &CanisterType) -> Option<Principal> {
        SUBNET_DIRECTORY.with_borrow_mut(|map| map.remove(ty))
    }

    //
    // Import & Export
    //

    pub fn import(view: SubnetDirectoryView) {
        SUBNET_DIRECTORY.with_borrow_mut(|map| {
            map.clear();
            for (ty, pid) in view {
                map.insert(ty, pid);
            }
        });
    }

    #[must_use]
    pub fn export() -> SubnetDirectoryView {
        SUBNET_DIRECTORY.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }
}
