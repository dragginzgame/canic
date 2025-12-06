pub mod icrc;
pub mod memory;
pub mod wasm;

use crate::model::{memory::MemoryError, wasm::WasmRegistryError};
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
