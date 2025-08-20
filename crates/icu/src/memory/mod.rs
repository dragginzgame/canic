pub mod memory_registry;

pub mod cells {
    mod app_state;
    mod canister_state;

    pub use app_state::*;
    pub use canister_state::*;
}

pub mod trackers {
    mod cycle_tracker;

    pub use cycle_tracker::*;
}

pub mod trees {
    mod child_index;
    mod subnet_directory;
    mod subnet_registry;

    pub use child_index::*;
    pub use subnet_directory::*;
    pub use subnet_registry::*;
}

pub use cells::*;
pub use memory_registry::*;
pub use trackers::*;
pub use trees::*;

use crate::ic::structures::{DefaultMemoryImpl, memory::MemoryManager};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// IDs
//

pub(crate) const MEMORY_REGISTRY_MEMORY_ID: u8 = 0;

pub(crate) const APP_STATE_MEMORY_ID: u8 = 1;
pub(crate) const CANISTER_STATE_MEMORY_ID: u8 = 2;
pub(crate) const SUBNET_REGISTRY_MEMORY_ID: u8 = 3;
pub(crate) const SUBNET_DIRECTORY_MEMORY_ID: u8 = 4;
pub(crate) const CHILD_INDEX_MEMORY_ID: u8 = 5;

pub(crate) const CYCLE_TRACKER_MEMORY_ID: u8 = 10;

//
// MEMORY_MANAGER
//

thread_local! {
    ///
    /// Define MEMORY_MANAGER thread-locally for the entire scope
    ///
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(
            DefaultMemoryImpl::default()
        ));
}

///
/// MemoryError
///

#[derive(Debug, ThisError)]
pub enum MemoryError {
    #[error(transparent)]
    MemoryRegistryError(#[from] MemoryRegistryError),

    #[error(transparent)]
    AppStateError(#[from] AppStateError),

    #[error(transparent)]
    CanisterStateError(#[from] CanisterStateError),

    #[error(transparent)]
    ChildIndexError(#[from] ChildIndexError),

    #[error(transparent)]
    SubnetDirectoryError(#[from] SubnetDirectoryError),

    #[error(transparent)]
    SubnetRegistryError(#[from] SubnetRegistryError),
}
