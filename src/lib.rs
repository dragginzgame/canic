pub mod auth;
pub mod config;
pub mod guard;
pub mod helper;
pub mod ic;
pub mod interface;
pub mod macros;
pub mod serialize;
pub mod state;

pub mod export {
    pub use ciborium;
    pub use defer;
}

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// Prelude
///

pub mod prelude {
    pub use crate::{
        Canister, Log,
        ic::{export_candid, init, query, update},
        icu_start, icu_start_root, log, perf,
        state::wasm::WasmManager,
    };
}

///
/// Error
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum Error {
    #[error(transparent)]
    AuthError(#[from] auth::AuthError),

    #[error(transparent)]
    ConfigError(#[from] config::ConfigError),

    #[error(transparent)]
    InterfaceError(#[from] interface::InterfaceError),

    #[error(transparent)]
    StateError(#[from] state::StateError),
}

///
/// Canister
///

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct Canister {
    path: String,
    is_sharded: bool,
}

impl Canister {
    #[must_use]
    pub fn new(path: &str, is_sharded: bool) -> Self {
        Self {
            path: path.to_string(),
            is_sharded,
        }
    }
}

///
/// Log
///

pub enum Log {
    Ok,
    Perf,
    Info,
    Warn,
    Error,
}

///
/// CYCLES
///

pub const KC: u128 = 1_000;
pub const MC: u128 = 1_000_000;
pub const BC: u128 = 1_000_000_000;
pub const TC: u128 = 1_000_000_000_000;
pub const QC: u128 = 1_000_000_000_000_000;
