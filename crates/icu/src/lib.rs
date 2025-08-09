pub mod auth;
pub mod canister;
pub mod config;
pub mod guard;
pub mod ic;
pub mod interface;
pub mod macros;
pub mod memory;
pub mod serialize;
pub mod state;
pub mod utils;

pub mod export {
    pub use defer;
}

pub use Error as IcuError;

use candid::CandidType;
use serde::Deserialize;
use thiserror::Error as ThisError;

///
/// Prelude
///

pub mod prelude {
    pub use crate::{
        Log, auth_require_all, auth_require_any,
        guard::{guard_query, guard_update},
        ic::{
            api::msg_caller,
            candid::CandidType,
            export_candid,
            icrc_ledger_types::icrc1::account::{Account, Subaccount},
            init,
            principal::Principal,
            query, update,
        },
        icu_config, icu_register_memory, icu_start, icu_start_root, log, perf, perf_start,
    };
}

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
    CanisterError(String),

    #[error("{0}")]
    ConfigError(String),

    #[error("{0}")]
    InterfaceError(String),

    #[error("{0}")]
    MemoryError(String),

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
from_to_string!(canister::CanisterError, CanisterError);
from_to_string!(config::ConfigError, ConfigError);
from_to_string!(interface::InterfaceError, InterfaceError);
from_to_string!(memory::MemoryError, MemoryError);
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
