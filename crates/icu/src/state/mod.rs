mod delegation_list;
mod icrc;
mod root;

pub use delegation_list::*;
pub use icrc::*;
pub use root::*;

use crate::ic::api::performance_counter;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

///
/// StateError
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum StateError {
    #[error(transparent)]
    CanisterRegistryError(#[from] CanisterRegistryError),

    #[error(transparent)]
    DelegationListError(#[from] DelegationListError),
}

thread_local! {
    pub static PERF_LAST: RefCell<u64> = RefCell::new(performance_counter(1));
}
