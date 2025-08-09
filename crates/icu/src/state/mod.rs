mod delegation_cache;
mod delegation_registry;
mod icrc;

pub use delegation_cache::*;
pub use delegation_registry::*;
pub use icrc::*;

use crate::ic::api::performance_counter;
use std::cell::RefCell;
use thiserror::Error as ThisError;

///
/// StateError
///

#[derive(Debug, ThisError)]
pub enum StateError {
    #[error(transparent)]
    DelegationRegistryError(#[from] DelegationRegistryError),
}

thread_local! {
    pub static PERF_LAST: RefCell<u64> = RefCell::new(performance_counter(1));
}
