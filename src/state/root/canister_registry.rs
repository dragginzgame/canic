use candid::CandidType;
use ic_cdk::println;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};
use thiserror::Error as ThisError;

///
/// CANISTER_REGISTRY
/// use Mutex to ensure thread safety for mutable access
///

pub static CANISTER_REGISTRY: LazyLock<Mutex<HashMap<String, Canister>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

///
/// CanisterRegistryError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum CanisterRegistryError {
    #[error("mutex lock failed")]
    LockFailed,

    #[error("canister '{0}' not found")]
    CanisterNotFound(String),
}

///
/// Canister
///

#[derive(Clone, Debug)]
pub struct Canister {
    pub def: CanisterDef,
    pub wasm: &'static [u8],
}

///
/// CanisterInfo
///

#[derive(CandidType, Clone, Debug)]
pub struct CanisterInfo {
    pub def: CanisterDef,
    pub wasm_size: usize,
}

impl From<&Canister> for CanisterInfo {
    fn from(canister: &Canister) -> Self {
        Self {
            def: canister.def.clone(),
            wasm_size: canister.wasm.len(),
        }
    }
}

///
/// CanisterDef
///

#[derive(CandidType, Clone, Debug)]
pub struct CanisterDef {
    pub auto_create: bool,
    pub is_sharded: bool,
}

///
/// CanisterRegistry
///

pub struct CanisterRegistry {}

impl CanisterRegistry {
    // get_canister
    pub fn get_canister(path: &str) -> Result<Canister, CanisterRegistryError> {
        let path = path.to_string();

        let canister = CANISTER_REGISTRY
            .lock()
            .map_err(|_| CanisterRegistryError::LockFailed)?
            .get(&path)
            .cloned()
            .ok_or(CanisterRegistryError::CanisterNotFound(path))?;

        Ok(canister)
    }

    // add_canister
    #[allow(clippy::cast_precision_loss)]
    pub fn add_canister(
        path: &str,
        def: &CanisterDef,
        wasm: &'static [u8],
    ) -> Result<(), CanisterRegistryError> {
        let path = path.to_string();

        CANISTER_REGISTRY
            .lock()
            .map_err(|_| CanisterRegistryError::LockFailed)?
            .insert(
                path.to_string(),
                Canister {
                    def: def.clone(),
                    wasm,
                },
            );

        println!("add_wasm: {} ({:.2} KB)", path, wasm.len() as f64 / 1000.0);

        Ok(())
    }

    // get_info
    pub fn get_info() -> Result<CanisterRegistryInfo, CanisterRegistryError> {
        let info = CANISTER_REGISTRY
            .lock()
            .map_err(|_| CanisterRegistryError::LockFailed)?
            .iter()
            .map(|(k, v)| (k.clone(), v.into()))
            .collect::<Vec<(String, CanisterInfo)>>();

        Ok(info)
    }
}

///
/// CanisterRegistryInfo
///

pub type CanisterRegistryInfo = Vec<(String, CanisterInfo)>;
