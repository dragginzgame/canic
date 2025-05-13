pub mod app;
pub mod canister;
pub mod counter;
pub mod subnet;

pub use counter::MemoryCounter;

use crate::{
    Log,
    ic::structures::{DefaultMemory, memory::MemoryId},
    icu_memory_manager, log,
    memory::{
        app::{AppMode, AppState, AppStateError},
        canister::{CanisterState, CanisterStateError, ChildIndex, ChildIndexError},
        subnet::{SubnetIndex, SubnetIndexError},
    },
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// MEMORY_MANAGER
//

icu_memory_manager!();

thread_local! {

    pub static APP_STATE: RefCell<AppState> = RefCell::new(
        allocate_state(|mem| AppState::init(mem, AppMode::Enabled))
    );

    pub static CANISTER_STATE: RefCell<CanisterState> = RefCell::new(
        allocate_state(CanisterState::init)
    );

    pub static CHILD_INDEX: RefCell<ChildIndex> = RefCell::new(
        allocate_state(ChildIndex::init)
    );

    pub static SUBNET_INDEX: RefCell<SubnetIndex> = RefCell::new(
        allocate_state(SubnetIndex::init)
    );

}

///
/// MemoryError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum MemoryError {
    #[error(transparent)]
    AppStateError(#[from] AppStateError),

    #[error(transparent)]
    CanisterStateError(#[from] CanisterStateError),

    #[error(transparent)]
    ChildIndexError(#[from] ChildIndexError),

    #[error(transparent)]
    SubnetIndexError(#[from] SubnetIndexError),
}

// allocate_state
pub fn allocate_state<T>(init_fn: impl FnOnce(DefaultMemory) -> T) -> T {
    let memory_id = MEMORY_COUNTER
        .with_borrow_mut(|this| this.next_memory_id())
        .unwrap();

    let memory = MEMORY_MANAGER.with_borrow(|mgr| mgr.get(MemoryId::new(memory_id)));

    log!(Log::Info, "allocating memory {memory_id}");

    init_fn(memory)
}
