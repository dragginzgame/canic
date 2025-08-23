pub mod wasm;

pub mod delegation {
    mod delegation_cache;
    mod delegation_registry;

    pub use delegation_cache::*;
    pub use delegation_registry::*;
}

pub mod icrc {
    mod icrc_10;
    mod icrc_21;

    pub use icrc_10::*;
    pub use icrc_21::*;
}

use crate::{
    ic::api::performance_counter,
    state::{delegation::DelegationRegistryError, wasm::WasmRegistryError},
};
use std::cell::RefCell;
use thiserror::Error as ThisError;

///
/// StateError
///

#[derive(Debug, ThisError)]
pub enum StateError {
    #[error(transparent)]
    DelegationRegistryError(#[from] DelegationRegistryError),

    #[error(transparent)]
    WasmRegistryError(#[from] WasmRegistryError),
}

thread_local! {
    pub static PERF_LAST: RefCell<u64> = RefCell::new(performance_counter(1));
}
