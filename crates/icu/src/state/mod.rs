pub mod root;
pub mod sharder;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

pub use root::canister_registry::CanisterRegistryError;

///
/// StateError
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum StateError {
    #[error(transparent)]
    CanisterRegistryError(#[from] CanisterRegistryError),
}
