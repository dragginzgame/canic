//! CANIC crate utilities for multi-canister apps on the Internet Computer.
pub mod auth;
pub mod cdk;
pub mod config;
pub mod env;
pub mod guard;
pub mod interface;
pub mod log;
pub mod macros;
pub mod memory;
pub mod ops;
pub mod runtime;
pub mod serialize;
pub mod spec;
pub mod state;
pub mod types;
pub mod utils;

pub mod export {
    pub use ::ctor;
    pub use ::defer;
}

pub use thiserror::Error as ThisError;

///
/// Prelude
/// should only be used in the Actor file
///

pub mod prelude {
    pub use crate::{
        Error as CanicError, auth_require_all, auth_require_any, canic_start, canic_start_root,
        cdk::{
            api::{canister_self, msg_caller},
            candid::CandidType,
            export_candid, init, query, update,
        },
        guard::{guard_query, guard_update},
        ic_memory, log, perf, perf_start,
        types::{CanisterType, Cycles},
    };
}

use crate::cdk::{
    call::{CallFailed, CandidDecodeFailed, Error as CallError},
    candid::{CandidType, Error as CandidError},
};
use serde::Deserialize;

///
/// Crate Version
///

pub const CRATE_NAME: &str = "canic";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

///
/// Error
///
/// top level error should handle all sub-errors, but not expose the child candid types
///

#[derive(CandidType, Debug, Deserialize, ThisError)]
pub enum Error {
    #[error("{0}")]
    CustomError(String),

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
    SerializeError(String),

    #[error("{0}")]
    StateError(String),

    ///
    /// Test Error
    /// as we don't want to import dev-dependencies
    ///

    #[error("{0}")]
    TestError(String),

    ///
    /// Common IC errors
    ///
    /// CallError          : should be automatic with ?
    /// CallFailed         : use this for wrapping <T, String> return values
    /// CandidError        : for decode_one errors etc.  automatic
    /// CandidDecodeFailed : automatic for calls like ::candid<T>()
    ///

    #[error("call error: {0}")]
    CallError(String),

    #[error("call failed: {0}")]
    CallFailed(String),

    #[error("candid error: {0}")]
    CandidError(String),

    #[error("candid decode failed: {0}")]
    CandidDecodeFailed(String),
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

impl Error {
    #[must_use]
    pub fn custom<S: Into<String>>(s: S) -> Self {
        Self::CustomError(s.into())
    }

    #[must_use]
    pub fn test<S: Into<String>>(s: S) -> Self {
        Self::TestError(s.into())
    }
}

from_to_string!(auth::AuthError, AuthError);
from_to_string!(config::ConfigError, ConfigError);
from_to_string!(env::EnvError, EnvError);
from_to_string!(interface::InterfaceError, InterfaceError);
from_to_string!(memory::MemoryError, MemoryError);
from_to_string!(ops::OpsError, OpsError);
from_to_string!(serialize::SerializeError, SerializeError);
from_to_string!(state::StateError, StateError);

from_to_string!(CallError, CallError);
from_to_string!(CallFailed, CallFailed);
from_to_string!(CandidDecodeFailed, CandidDecodeFailed);
from_to_string!(CandidError, CandidError);
