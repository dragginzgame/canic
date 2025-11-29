pub mod icrc;
pub mod memory;
pub mod wasm;

use crate::{
    cdk::api::performance_counter,
    model::{memory::MemoryError, wasm::WasmRegistryError},
};
use std::cell::RefCell;
use thiserror::Error as ThisError;

///
/// ModelError
///

#[derive(Debug, ThisError)]
pub enum ModelError {
    #[error(transparent)]
    MemoryError(#[from] MemoryError),

    #[error(transparent)]
    WasmRegistryError(#[from] WasmRegistryError),
}

thread_local! {
    pub static PERF_LAST: RefCell<u64> = RefCell::new(performance_counter(1));
}
