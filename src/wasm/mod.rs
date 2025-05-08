use candid::CandidType;
use ic_cdk::println;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};
use thiserror::Error as ThisError;

///
/// WASM_FILES
/// use Mutex to ensure thread safety for mutable access
///

pub static WASM_FILES: LazyLock<Mutex<HashMap<CanisterType, &'static [u8]>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

///
/// WasmError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum WasmError {
    #[error("mutex lock failed")]
    LockFailed,

    #[error("wasm not found for canister type {0}")]
    WasmNotFound(CanisterType),
}

///
/// WasmManager
///

pub struct WasmManager {}

impl WasmManager {
    // get_wasm
    pub fn get_wasm(ty: &CanisterType) -> Result<&'static [u8], WasmError> {
        let file = WASM_FILES
            .lock()
            .map_err(|_| WasmError::LockFailed)?
            .get(ty)
            .copied()
            .ok_or_else(|| WasmError::WasmNotFound(ty.clone()))?;

        Ok(file)
    }

    // add_wasm
    #[allow(clippy::cast_precision_loss)]
    pub fn add_wasm(ty: CanisterType, wasm: &'static [u8]) -> Result<(), WasmError> {
        WASM_FILES
            .lock()
            .map_err(|_| WasmError::LockFailed)?
            .insert(ty.clone(), wasm);

        println!("add_wasm: {} ({:.2} KB)", ty, wasm.len() as f64 / 1000.0);

        Ok(())
    }

    // info
    pub fn info() -> Result<Vec<(CanisterType, usize)>, WasmError> {
        let info = WASM_FILES
            .lock()
            .map_err(|_| WasmError::LockFailed)?
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect();

        Ok(info)
    }
}
