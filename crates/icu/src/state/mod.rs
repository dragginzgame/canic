mod icrc;
mod root;
mod session_registry;

pub use icrc::*;
pub use root::*;
pub use session_registry::*;

use crate::ic::api::performance_counter;
use std::cell::RefCell;
use thiserror::Error as ThisError;

///
/// StateError
///

#[derive(Debug, ThisError)]
pub enum StateError {
    #[error(transparent)]
    CanisterRegistryError(#[from] CanisterRegistryError),

    #[error(transparent)]
    SessionRegistryError(#[from] SessionRegistryError),
}

thread_local! {
    pub static PERF_LAST: RefCell<u64> = RefCell::new(performance_counter(1));
}
