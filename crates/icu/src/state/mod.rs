pub mod root;
pub mod sharder;

use crate::ic::api::performance_counter;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
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

thread_local! {
    pub static PERF_LAST: RefCell<u64> = RefCell::new(performance_counter(1));
}
