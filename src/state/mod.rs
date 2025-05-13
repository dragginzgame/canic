pub mod auth;
pub mod core;
pub mod root;
pub mod sharder;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

pub use {
    core::{AppStateError, CanisterStateError, ChildIndexError, SubnetIndexError},
    root::canister_registry::CanisterRegistryError,
};

///
/// StateError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum StateError {
    // core
    #[error(transparent)]
    AppStateError(#[from] AppStateError),

    #[error(transparent)]
    CanisterStateError(#[from] CanisterStateError),

    #[error(transparent)]
    ChildIndexError(#[from] ChildIndexError),

    #[error(transparent)]
    SubnetIndexError(#[from] SubnetIndexError),

    // root
    #[error(transparent)]
    CanisterRegistryError(#[from] CanisterRegistryError),
}
