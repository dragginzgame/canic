pub mod app_state;
pub mod canister;
pub mod cycle_tracker;
pub mod memory_registry;

pub use app_state::{AppState, AppStateData};
pub use canister::{
    children::{CanisterChildren, CanisterChildrenView},
    directory::{CanisterDirectory, CanisterDirectoryView},
    partition::{PartitionEntry, PartitionRegistry, PartitionRegistryView},
    pool::{CanisterPool, CanisterPoolView},
    registry::{CanisterRegistry, CanisterRegistryView},
    state::{CanisterState, CanisterStateData},
};
pub use cycle_tracker::{CycleTracker, CycleTrackerView};
pub use memory_registry::MemoryRegistry;

use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::MemoryManager},
    memory::{
        app_state::AppStateError,
        canister::{
            children::CanisterChildrenError, directory::CanisterDirectoryError,
            registry::CanisterRegistryError, state::CanisterStateError,
        },
        memory_registry::MemoryRegistryError,
    },
};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// MEMORY_IDs
//

pub(crate) const MEMORY_REGISTRY_MEMORY_ID: u8 = 0;

// root
pub(crate) const CANISTER_POOL_MEMORY_ID: u8 = 1;
pub(crate) const CANISTER_REGISTRY_MEMORY_ID: u8 = 2;

// root-authoritative (cascaded to subnet)
pub(crate) const APP_STATE_MEMORY_ID: u8 = 3;
pub(crate) const CANISTER_DIRECTORY_MEMORY_ID: u8 = 4;

// all
pub(crate) const CANISTER_STATE_MEMORY_ID: u8 = 5;
pub(crate) const CANISTER_CHILDREN_MEMORY_ID: u8 = 6;
pub(crate) const PARTITION_REGISTRY_MEMORY_ID: u8 = 7;
pub(crate) const PARTITION_ITEM_MAP_MEMORY_ID: u8 = 8;

// trackers (all)
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
    AppStateError(#[from] AppStateError),

    #[error(transparent)]
    CanisterChildrenError(#[from] CanisterChildrenError),

    #[error(transparent)]
    CanisterDirectoryError(#[from] CanisterDirectoryError),

    #[error(transparent)]
    CanisterRegistryError(#[from] CanisterRegistryError),

    #[error(transparent)]
    CanisterStateError(#[from] CanisterStateError),

    #[error(transparent)]
    MemoryRegistryError(#[from] MemoryRegistryError),

    #[error(transparent)]
    PartitionRegistryError(#[from] canister::partition::PartitionRegistryError),
}
