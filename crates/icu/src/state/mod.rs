mod delegation_list;
mod root;

use crate::ic::api::performance_counter;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

pub use delegation_list::{
    Delegation, DelegationList, DelegationListError, RegisterDelegationArgs,
};
pub use root::{
    CanisterAttributes, CanisterData, CanisterRegistry, CanisterRegistryData, CanisterRegistryError,
};

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
