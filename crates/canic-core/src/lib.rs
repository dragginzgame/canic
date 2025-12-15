//! CANIC crate utilities for multi-canister apps on the Internet Computer.
pub mod access;

// -----------------------------------------------------------------------------
// Phase 0: path coherence re-exports (no behavior change)
// -----------------------------------------------------------------------------

pub use access::{auth, guard, policy};
pub mod config;
pub mod dispatch;
pub mod dto;
pub mod env;
pub mod ids;
pub mod log;
pub mod macros;
pub(crate) mod model;
pub mod ops;
pub mod perf;
pub mod spec;

pub use ::canic_cdk as cdk;
pub use ::canic_memory as memory;
pub use ::canic_memory::{eager_init, eager_static, ic_memory, ic_memory_range};
pub use ::canic_types as types;
pub use ::canic_utils as utils;

/// Internal re-exports required for macro expansion.
/// Not part of the public API.
#[doc(hidden)]
pub mod __reexports {
    pub use ::ctor;
    pub use ::defer;
}

pub use thiserror::Error as ThisError;

use crate::cdk::{
    call::{CallFailed, CandidDecodeFailed, Error as CallError},
    candid::{CandidType, Error as CandidError},
};
use serde::Deserialize;

///
/// Crate Version
///

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

///
/// Error
///
/// top level error should handle all sub-errors, but not expose the child candid types
///

#[derive(CandidType, Debug, Deserialize, ThisError)]
pub enum Error {
    #[error("{0}")]
    AccessError(String),

    #[error("{0}")]
    ConfigError(String),

    #[error("{0}")]
    CustomError(String),

    #[error("{0}")]
    ModelError(String),

    #[error("{0}")]
    OpsError(String),

    #[error("{0}")]
    SerializeError(String),

    #[error("http request failed: {0}")]
    HttpRequest(String),

    #[error("http error status: {0}")]
    HttpErrorCode(u32),

    #[error("http decode failed: {0}")]
    HttpDecode(String),

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
    /// Build a custom error from a string without defining a new variant.
    #[must_use]
    pub fn custom<S: Into<String>>(s: S) -> Self {
        Self::CustomError(s.into())
    }

    /// Build a test error to avoid extra dev-only dependencies.
    #[must_use]
    pub fn test<S: Into<String>>(s: S) -> Self {
        Self::TestError(s.into())
    }
}

from_to_string!(access::AccessError, AccessError);
from_to_string!(config::ConfigError, ConfigError);
from_to_string!(model::ModelError, ModelError);
from_to_string!(ops::OpsError, OpsError);
from_to_string!(serde_json::Error, HttpDecode);

from_to_string!(CallError, CallError);
from_to_string!(CallFailed, CallFailed);
from_to_string!(CandidDecodeFailed, CandidDecodeFailed);
from_to_string!(CandidError, CandidError);
