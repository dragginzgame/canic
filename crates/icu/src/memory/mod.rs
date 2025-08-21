pub mod app_state;
pub mod canister_pool;
pub mod canister_state;
pub mod child_index;
pub mod cycle_tracker;
pub mod memory_registry;
pub mod subnet_directory;
pub mod subnet_registry;

pub use app_state::{AppState, AppStateData};
pub use canister_pool::{CanisterPool, CanisterPoolView};
pub use canister_state::{CanisterState, CanisterStateData};
pub use child_index::{ChildIndex, ChildIndexView};
pub use cycle_tracker::{CycleTracker, CycleTrackerView};
pub use memory_registry::{MemoryRegistry, MemoryRegistryView};
pub use subnet_directory::{SubnetDirectory, SubnetDirectoryView};
pub use subnet_registry::{SubnetRegistry, SubnetRegistryView};

use crate::{
    ic::structures::{DefaultMemoryImpl, memory::MemoryManager},
    memory::{
        app_state::AppStateError, canister_state::CanisterStateError, child_index::ChildIndexError,
        memory_registry::MemoryRegistryError, subnet_directory::SubnetDirectoryError,
        subnet_registry::SubnetRegistryError,
    },
};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// MEMORY_IDs
//

pub(crate) const MEMORY_REGISTRY_MEMORY_ID: u8 = 0;

pub(crate) const APP_STATE_MEMORY_ID: u8 = 1;
pub(crate) const CANISTER_POOL_MEMORY_ID: u8 = 2;
pub(crate) const CANISTER_STATE_MEMORY_ID: u8 = 3;
pub(crate) const CHILD_INDEX_MEMORY_ID: u8 = 4;
pub(crate) const SUBNET_DIRECTORY_MEMORY_ID: u8 = 5;
pub(crate) const SUBNET_REGISTRY_MEMORY_ID: u8 = 6;

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
