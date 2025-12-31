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
pub(crate) mod infra;
pub mod lifecycle;
pub mod log;
pub mod macros;
pub(crate) mod model;
pub(crate) mod ops;
pub mod perf;
pub mod policy;
pub mod workflow;

pub use {
    ::canic_cdk as cdk,
    ::canic_memory as memory,
    ::canic_memory::{eager_init, eager_static, ic_memory, ic_memory_range},
    ::canic_utils as utils,
    dto::error::{Error as PublicError, ErrorCode},
    thiserror::Error as ThisError,
};

/// Internal re-exports required for macro expansion.
/// Not part of the public API.
#[doc(hidden)]
pub mod __reexports {
    pub use ::ctor;
}

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
pub(crate) enum Error {
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
}

impl Error {
    pub fn public(&self) -> PublicError {
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
        }
    }

    fn public_message(code: ErrorCode, message: &'static str) -> PublicError {
        PublicError {
            code,
            message: message.to_string(),
        }
    }

    fn public_http_status(status: u32) -> PublicError {
        let code = match status {
            401 | 403 => ErrorCode::Unauthorized,
            404 => ErrorCode::NotFound,
            409 => ErrorCode::Conflict,
            429 => ErrorCode::ResourceExhausted,
            400..=499 => ErrorCode::InvalidInput,
            500..=599 => ErrorCode::Internal,
            _ => ErrorCode::Internal,
        };

        PublicError {
            code,
            message: format!("http status {status}"),
        }
    }
}

impl From<&Error> for PublicError {
    fn from(err: &Error) -> Self {
        err.public()
    }
}

impl From<Error> for PublicError {
    fn from(err: Error) -> Self {
        Self::from(&err)
    }
}
