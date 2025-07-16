pub mod app;
pub mod canister;
pub mod registry;
pub mod subnet;

pub use app::{AppState, AppStateData};
pub use canister::{CanisterState, CanisterStateData, ChildIndex, ChildIndexData};
pub use registry::{Registry, RegistryData, RegistryError};
pub use subnet::{SubnetIndex, SubnetIndexData};

use crate::{
    ic::structures::{
        DefaultMemoryImpl,
        memory::{MemoryId, MemoryManager},
    },
    memory::{
        app::AppStateError,
        canister::{CanisterStateError, ChildIndexError},
        subnet::SubnetIndexError,
    },
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// IDs
//

pub(crate) const MEMORY_REGISTRY_ID: u8 = 0;

pub(crate) const APP_STATE_MEMORY_ID: u8 = 1;
pub(crate) const CANISTER_STATE_MEMORY_ID: u8 = 2;
pub(crate) const CHILD_INDEX_MEMORY_ID: u8 = 3;
pub(crate) const SUBNET_INDEX_MEMORY_ID: u8 = 4;

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

    ///
    /// MEMORY_REGISTRY
    ///
    pub static MEMORY_REGISTRY: RefCell<Registry> =
        RefCell::new(<Registry>::init(
            MEMORY_MANAGER.with_borrow(|this| {
                    this.get(MemoryId::new(MEMORY_REGISTRY_ID))
                }
            ),
        ));

}

///
/// Helpers
///

#[must_use]
pub fn memory_registry_data() -> RegistryData {
    MEMORY_REGISTRY.with_borrow(|state| state.as_data())
}

///
/// MemoryError
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum MemoryError {
    // registry
    #[error(transparent)]
    RegistryError(#[from] RegistryError),

    // others
    #[error(transparent)]
    AppStateError(#[from] AppStateError),

    #[error(transparent)]
    CanisterStateError(#[from] CanisterStateError),

    #[error(transparent)]
    ChildIndexError(#[from] ChildIndexError),

    #[error(transparent)]
    SubnetIndexError(#[from] SubnetIndexError),
}
