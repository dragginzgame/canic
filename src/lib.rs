pub mod cycles;
pub mod helper;
pub mod wasm;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// Error
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum Error {
    #[error(transparent)]
    WasmError(#[from] wasm::WasmError),
}
