pub mod auth;
pub mod cdk;
pub mod config;
pub mod env;
pub mod guard;
pub mod interface;
pub mod macros;
pub mod memory;
pub mod ops;
pub mod serialize;
pub mod spec;
pub mod state;
pub mod types;
pub mod utils;

pub mod export {
    pub use defer;
}

pub use Error as IcuError;

///
/// Prelude
///

pub mod prelude {
    pub use crate::{
        Log, auth_require_all, auth_require_any,
        cdk::{
            api::msg_caller, candid::CandidType, export_candid, init, principal::Principal, query,
            update,
        },
        guard::{guard_query, guard_update},
        icu_register_memory, icu_start, icu_start_root, log, perf, perf_start,
        types::{CanisterType, Cycles},
    };
}

use candid::CandidType;
use serde::Deserialize;
use std::time::Duration;
use thiserror::Error as ThisError;
use types::CanisterType;

///
/// Crate Version
///

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

///
/// Constants
///

pub const CANISTER_INIT_DELAY: Duration = Duration::new(5, 0);

///
/// Icu Canisters
///

pub const TEST: CanisterType = CanisterType::new("test");

///
/// Error
///
/// top level error should handle all sub-errors, but not expose the child candid types
///

#[derive(CandidType, Debug, Deserialize, ThisError)]
pub enum Error {
    #[error("{0}")]
    AuthError(String),

    #[error("{0}")]
    ConfigError(String),

    #[error("{0}")]
    EnvError(String),

    #[error("{0}")]
    InterfaceError(String),

    #[error("{0}")]
    MemoryError(String),

    #[error("{0}")]
    OpsError(String),

    #[error("{0}")]
    StateError(String),
}

macro_rules! from_to_string {
    ($from:ty, $variant:ident) => {
        impl From<$from> for Error {
            fn from(e: $from) -> Self {
                Error::$variant(e.to_string())
            }
        }
    };
}

from_to_string!(auth::AuthError, AuthError);
from_to_string!(config::ConfigError, ConfigError);
from_to_string!(env::EnvError, EnvError);
from_to_string!(interface::InterfaceError, InterfaceError);
from_to_string!(memory::MemoryError, MemoryError);
from_to_string!(ops::OpsError, OpsError);
from_to_string!(state::StateError, StateError);

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
