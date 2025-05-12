use candid::CandidType;
use ic_cdk::println;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};
use thiserror::Error as ThisError;

///
/// WASMS
/// use Mutex to ensure thread safety for mutable access
///

pub static WASMS: LazyLock<Mutex<HashMap<String, &'static [u8]>>> =
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
    pub fn get_wasm(path: &str) -> Result<&'static [u8], WasmError> {
        let path = path.to_string();

        let file = WASMS
            .lock()
            .map_err(|_| WasmError::LockFailed)?
            .get(&path)
            .copied()
            .ok_or(WasmError::WasmNotFound(path))?;

        Ok(file)
    }

    // add_wasm
    #[allow(clippy::cast_precision_loss)]
    pub fn add_wasm(path: &str, wasm: &'static [u8]) -> Result<(), WasmError> {
        let path = path.to_string();

        WASMS
            .lock()
            .map_err(|_| WasmError::LockFailed)?
            .insert(path.clone(), wasm);

        println!("add_wasm: {} ({:.2} KB)", path, wasm.len() as f64 / 1000.0);

        Ok(())
    }

    // add_wasms
    #[allow(clippy::cast_precision_loss)]
    pub fn add_wasms(wasms: &[(&str, &'static [u8])]) -> Result<(), WasmError> {
        for (path, wasm) in wasms {
            Self::add_wasm(path, wasm)?;
        }

        Ok(())
    }

    // info
    pub fn info() -> Result<Vec<(String, usize)>, WasmError> {
        let info = WASMS
            .lock()
            .map_err(|_| WasmError::LockFailed)?
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect();

        Ok(info)
    }
}

///
/// WasmInfoData
///

pub type WasmInfoData = Vec<(String, usize)>;
