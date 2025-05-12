pub mod auth;
pub mod core;
pub mod sharder;
pub mod wasm;

use crate::memory_manager;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

pub use {
    core::{
        app_state::AppStateError, canister_state::CanisterStateError, child_index::ChildIndexError,
        subnet_index::SubnetIndexError,
    },
    wasm::WasmError,
};

//
// MEMORY_MANAGER
//

memory_manager!();

// global memory ids are hardcoded in one place
const APP_STATE_MEMORY_ID: u8 = 1;
const SUBNET_INDEX_MEMORY_ID: u8 = 2;
const CANISTER_STATE_MEMORY_ID: u8 = 3;
const CHILD_INDEX_MEMORY_ID: u8 = 4;

///
/// StateError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum StateError {
    #[error(transparent)]
    AppStateError(#[from] AppStateError),

    #[error(transparent)]
    CanisterStateError(#[from] CanisterStateError),

    #[error(transparent)]
    ChildIndexError(#[from] ChildIndexError),

    #[error(transparent)]
    SubnetIndexError(#[from] SubnetIndexError),

    #[error(transparent)]
    WasmError(#[from] WasmError),
}
