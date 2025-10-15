pub mod icrc;
pub mod wasm;

use crate::{cdk::api::performance_counter, state::wasm::WasmRegistryError};
use std::cell::RefCell;
use thiserror::Error as ThisError;

///
/// StateError
///

#[derive(Debug, ThisError)]
pub enum StateError {
    #[error(transparent)]
    WasmRegistryError(#[from] WasmRegistryError),
}

thread_local! {
    pub static PERF_LAST: RefCell<u64> = RefCell::new(performance_counter(1));
}
