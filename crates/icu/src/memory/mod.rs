pub mod app;
pub mod canister;
pub mod cycle;
pub mod memory_registry;
pub mod shard;
pub mod subnet;

pub use app::{AppState, AppStateData};
pub use canister::{
    CanisterPool, CanisterPoolView, CanisterState, CanisterStateData, CanisterStateError,
};
pub use cycle::{CycleTracker, CycleTrackerView};
pub use memory_registry::MemoryRegistry;
pub use shard::{ShardRegistry, ShardRegistryError, ShardRegistryView};
pub use subnet::{CanisterEntry, CanisterStatus, SubnetDirectory, SubnetError, SubnetRegistry};

use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::MemoryManager},
    memory::{app::AppStateError, memory_registry::MemoryRegistryError},
};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// MEMORY_IDs
//

pub(crate) const MEMORY_REGISTRY_MEMORY_ID: u8 = 0;

// root
pub(crate) const CANISTER_POOL_MEMORY_ID: u8 = 1;
pub(crate) const SUBNET_REGISTRY_MEMORY_ID: u8 = 2;

// root-authoritative (cascaded to subnet)
pub(crate) const APP_STATE_MEMORY_ID: u8 = 5;
pub(crate) const SUBNET_CHILDREN_MEMORY_ID: u8 = 6;
pub(crate) const SUBNET_DIRECTORY_MEMORY_ID: u8 = 7;
pub(crate) const SUBNET_PARENTS_MEMORY_ID: u8 = 8;

// all
pub(crate) const CANISTER_STATE_MEMORY_ID: u8 = 10;
pub(crate) const SHARD_REGISTRY_MEMORY_ID: u8 = 11;
pub(crate) const SHARD_TENANT_MAP_MEMORY_ID: u8 = 12;

// trackers (all)
pub(crate) const CYCLE_TRACKER_MEMORY_ID: u8 = 15;

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
    CanisterStateError(#[from] CanisterStateError),

    #[error(transparent)]
    MemoryRegistryError(#[from] MemoryRegistryError),

    #[error(transparent)]
    ShardRegistryError(#[from] ShardRegistryError),

    #[error(transparent)]
    SubnetError(#[from] SubnetError),
}
