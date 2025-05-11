pub mod auth;
pub mod core;
pub mod sharder;
pub mod wasm;

use crate::memory_manager;

//
// MEMORY_MANAGER
//

memory_manager!();

// global memory ids are hardcoded in one place
const APP_STATE_MEMORY_ID: u8 = 1;
const SUBNET_INDEX_MEMORY_ID: u8 = 2;
const CANISTER_STATE_MEMORY_ID: u8 = 3;
const CHILD_INDEX_MEMORY_ID: u8 = 4;

//#[error(transparent)]
//WasmError(#[from] wasm::WasmError),
