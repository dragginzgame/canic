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

pub static WASM_FILES: LazyLock<Mutex<HashMap<String, &'static [u8]>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

///
/// WasmError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum WasmError {
    #[error("mutex lock failed")]
    LockFailed,

    #[error("wasm '{0}' not found")]
    WasmNotFound(String),
}

///
/// WasmManager
///

pub struct WasmManager {}

impl WasmManager {
    // get_wasm
    pub fn get_wasm(name: &str) -> Result<&'static [u8], WasmError> {
        let file = WASM_FILES
            .lock()
            .map_err(|_| WasmError::LockFailed)?
            .get(name)
            .copied()
            .ok_or_else(|| WasmError::WasmNotFound(name.to_string()))?;

        Ok(file)
    }

    // add_wasm
    #[allow(clippy::cast_precision_loss)]
    pub fn add_wasm(name: String, wasm: &'static [u8]) -> Result<(), WasmError> {
        WASM_FILES
            .lock()
            .map_err(|_| WasmError::LockFailed)?
            .insert(name.clone(), wasm);

        println!("add_wasm: {} ({:.2} KB)", name, wasm.len() as f64 / 1000.0);

        Ok(())
    }

    // add_wasms
    #[allow(clippy::cast_precision_loss)]
    pub fn add_wasms(wasms: &[(&'static str, &'static [u8])]) -> Result<(), WasmError> {
        for (ty, wasm) in wasms {
            Self::add_wasm(ty.to_string(), wasm)?;
        }

        Ok(())
    }

    // info
    pub fn info() -> Result<Vec<(String, usize)>, WasmError> {
        let info = WASM_FILES
            .lock()
            .map_err(|_| WasmError::LockFailed)?
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect();

        Ok(info)
    }
}
