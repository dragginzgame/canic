pub mod app;
pub mod canister;
pub mod registry;
pub mod subnet;

pub use registry::{Registry, RegistryError};

use crate::{
    MEMORY_REGISTRY_ID,
    ic::structures::{
        DefaultMemoryImpl,
        memory::{MemoryId, MemoryManager},
    },
    icu_register_memory,
    memory::{
        app::{AppState, AppStateError},
        canister::{CanisterState, CanisterStateError, ChildIndex, ChildIndexError},
        subnet::{SubnetIndex, SubnetIndexError},
    },
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

pub fn init() {
    APP_STATE.with(|_| {});
    CANISTER_STATE.with(|_| {});
    CHILD_INDEX.with(|_| {});
    SUBNET_INDEX.with(|_| {});
}

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

    pub static APP_STATE: RefCell<AppState> = RefCell::new(
        icu_register_memory!(AppState, 1, AppState::init)
    );

    pub static CANISTER_STATE: RefCell<CanisterState> = RefCell::new(
        icu_register_memory!(CanisterState, 2, CanisterState::init)
    );

    pub static CHILD_INDEX: RefCell<ChildIndex> = RefCell::new(
        icu_register_memory!(ChildIndex, 3, ChildIndex::init)
    );

    pub static SUBNET_INDEX: RefCell<SubnetIndex> = RefCell::new(
        icu_register_memory!(SubnetIndex, 4, SubnetIndex::init)
    );

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
