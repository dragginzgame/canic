//! Core Canic library used inside canisters.
//!
//! Most users should depend on the `canic` facade crate, which re-exports this crate
//! under `canic::core` and exposes the common entrypoint macros:
//! - `canic::build!` / `canic::build_root!` (in `build.rs`) to validate/embed `canic.toml`
//! - `canic::start!` / `canic::start_root!` (in `lib.rs`) to wire lifecycle hooks and export endpoints
//!
//! ## Layering
//!
//! Canic is organized to keep endpoint code thin and policies centralized:
//! - `access/` contains guard/auth/rule helpers for boundary enforcement.
//! - `workflow/` implements orchestration and lifecycle workflows.
//! - `policy/` owns deterministic decision rules.
//! - `ops/` provides mechanical, reusable side-effecting operations.
//! - `model/` owns storage (stable memory) and in-process registries/caches.
//! - `macros/` provides public macro entrypoints and endpoint bundles.
//!
//! The default flow is: endpoints → workflow → policy → ops → model.

// -----------------------------------------------------------------------------
// Phase 0: path coherence re-exports (no behavior change)
// -----------------------------------------------------------------------------

pub mod access;
pub mod api;
pub mod config;
pub mod dispatch;
pub mod dto;
pub mod ids;
pub mod infra;
pub mod lifecycle;
pub mod log;
pub mod macros;
pub(crate) mod model;
pub mod ops;
pub mod perf;
pub mod policy;
pub mod workflow;

pub use {
    ::canic_cdk as cdk,
    ::canic_memory as memory,
    ::canic_memory::{eager_init, eager_static, ic_memory, ic_memory_range},
    ::canic_utils as utils,
    dto::error::{Error, ErrorCode},
    thiserror::Error as ThisError,
};

/// Internal re-exports required for macro expansion.
/// Not part of the public API.
#[doc(hidden)]
pub mod __reexports {
    pub use ::ctor;
}

use crate::cdk::{
    call::{CallFailed, CandidDecodeFailed, Error as CallError},
    candid::Error as CandidError,
};

///
/// Crate Version
///

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

///
/// Error
///
/// Internal, structured error type.
///
/// This error:
/// - is NOT Candid-exposed
/// - is NOT stable across versions
/// - may evolve freely
///
/// All canister endpoints must convert this into a public error envelope.
///

#[derive(Debug, ThisError)]
pub enum CanicError {
    #[error(transparent)]
    Access(#[from] access::AccessError),

    #[error(transparent)]
    Config(#[from] config::ConfigError),

    #[error(transparent)]
    Infra(#[from] infra::InfraError),

    #[error(transparent)]
    Model(#[from] model::ModelError),

    #[error(transparent)]
    Ops(#[from] ops::OpsError),

    #[error(transparent)]
    Policy(#[from] policy::PolicyError),

    #[error(transparent)]
    Workflow(#[from] workflow::WorkflowError),

    // ---------------------------------------------------------------------
    // HTTP / networking
    // ---------------------------------------------------------------------
    #[error("http request failed: {0}")]
    HttpRequest(String),

    #[error("http error status: {0}")]
    HttpStatus(u32),

    #[error("http decode failed: {0}")]
    HttpDecode(#[from] serde_json::Error),

    // ---------------------------------------------------------------------
    // IC / Candid
    // ---------------------------------------------------------------------
    #[error(transparent)]
    Call(#[from] CallError),

    #[error(transparent)]
    CallFailed(#[from] CallFailed),

    #[error(transparent)]
    Candid(#[from] CandidError),

    #[error(transparent)]
    CandidDecode(#[from] CandidDecodeFailed),

    // ---------------------------------------------------------------------
    // Utility / test-only
    // ---------------------------------------------------------------------
    #[error("test error: {0}")]
    Test(String),

    #[error("custom error: {0}")]
    Custom(String),
}

impl CanicError {
    pub fn public(&self) -> Error {
        match self {
            // ---------------------------------------------------------
            // Access / authorization
            // ---------------------------------------------------------
            Self::Access(_) => Self::public_message(ErrorCode::Unauthorized, "unauthorized"),

            // ---------------------------------------------------------
            // Input / configuration
            // ---------------------------------------------------------
            Self::Config(_) => {
                Self::public_message(ErrorCode::InvalidInput, "invalid configuration")
            }
            Self::Candid(_) | Self::CandidDecode(_) => {
                Self::public_message(ErrorCode::InvalidInput, "invalid input")
            }

            // ---------------------------------------------------------
            // Policy decisions
            // ---------------------------------------------------------
            Self::Policy(_) => Self::public_message(ErrorCode::Conflict, "policy rejected"),

            // ---------------------------------------------------------
            // State / invariants
            // ---------------------------------------------------------
            Self::Model(_) => {
                Self::public_message(ErrorCode::InvariantViolation, "invariant violation")
            }

            // ---------------------------------------------------------
            // Infrastructure / execution
            // ---------------------------------------------------------
            Self::Infra(_) | Self::Ops(_) | Self::Workflow(_) => {
                Self::public_message(ErrorCode::Internal, "internal error")
            }
            Self::Call(_) | Self::CallFailed(_) => {
                Self::public_message(ErrorCode::Internal, "ic call failed")
            }

            // ---------------------------------------------------------
            // HTTP
            // ---------------------------------------------------------
            Self::HttpStatus(code) => Self::public_http_status(*code),
            Self::HttpRequest(_) => {
                Self::public_message(ErrorCode::Internal, "http request failed")
            }
            Self::HttpDecode(_) => Self::public_message(ErrorCode::Internal, "http decode failed"),

            // ---------------------------------------------------------
            // Fallbacks
            // ---------------------------------------------------------
            Self::Custom(msg) => Error {
                code: ErrorCode::Internal,
                message: msg.clone(),
            },
            Self::Test(msg) => Error {
                code: ErrorCode::Internal,
                message: msg.clone(),
            },
        }
    }

    fn public_message(code: ErrorCode, message: &'static str) -> Error {
        Error {
            code,
            message: message.to_string(),
        }
    }

    fn public_http_status(status: u32) -> Error {
        let code = match status {
            401 | 403 => ErrorCode::Unauthorized,
            404 => ErrorCode::NotFound,
            409 => ErrorCode::Conflict,
            429 => ErrorCode::ResourceExhausted,
            400..=499 => ErrorCode::InvalidInput,
            500..=599 => ErrorCode::Internal,
            _ => ErrorCode::Internal,
        };

        Error {
            code,
            message: format!("http status {status}"),
        }
    }

    /// Build a custom error without introducing a new variant.
    #[must_use]
    pub fn custom<S: Into<String>>(msg: S) -> Self {
        Self::Custom(msg.into())
    }

    /// Test-only helper to avoid dev-dependencies.
    #[must_use]
    pub fn test<S: Into<String>>(msg: S) -> Self {
        Self::Test(msg.into())
    }
}

impl From<&CanicError> for Error {
    fn from(err: &CanicError) -> Self {
        err.public()
    }
}

impl From<CanicError> for Error {
    fn from(err: CanicError) -> Self {
        Error::from(&err)
    }
}
