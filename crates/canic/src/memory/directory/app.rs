use crate::{
    Error, ThisError,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    memory::{MemoryError, directory::DirectoryError, id::directory::APP_DIRECTORY_ID},
    types::{CanisterType, Principal},
};
use std::cell::RefCell;

//
// APP_DIRECTORY
//

eager_static! {
    static APP_DIRECTORY: RefCell<BTreeMap<CanisterType, Principal, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(AppDirectory, APP_DIRECTORY_ID)));
}

///
/// AppDirectoryError
///

#[derive(Debug, ThisError)]
pub enum AppDirectoryError {
    #[error("canister already in app directory: {0}")]
    AlreadyRegistered(CanisterType),

    #[error("canister not in app directory: {0}")]
    TypeNotFound(CanisterType),
}

impl From<AppDirectoryError> for Error {
    fn from(err: AppDirectoryError) -> Self {
        MemoryError::from(DirectoryError::from(err)).into()
    }
}

///
/// AppDirectory
///

pub struct AppDirectory;

pub type AppDirectoryView = Vec<(CanisterType, Principal)>;

impl AppDirectory {
    #[must_use]
    pub fn get(ty: &CanisterType) -> Option<Principal> {
        APP_DIRECTORY.with_borrow(|map| map.get(ty))
    }

    pub fn try_get(ty: &CanisterType) -> Result<Principal, Error> {
        Self::get(ty).ok_or_else(|| AppDirectoryError::TypeNotFound(ty.clone()).into())
    }

    #[must_use]
    pub fn remove(ty: &CanisterType) -> Option<Principal> {
        APP_DIRECTORY.with_borrow_mut(|map| map.remove(ty))
    }

    //
    // Import & Export
    //

    pub fn import(view: AppDirectoryView) {
        APP_DIRECTORY.with_borrow_mut(|map| {
            map.clear();
            for (ty, pid) in view {
                map.insert(ty, pid);
            }
        });
    }

    #[must_use]
    pub fn export() -> AppDirectoryView {
        APP_DIRECTORY.with_borrow(|map| {
            map.iter()
                .map(|entry| (entry.key().clone(), entry.value()))
                .collect()
        })
    }
}
