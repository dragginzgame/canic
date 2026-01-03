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
#[doc(hidden)]
pub mod dispatch;
pub mod domain;
pub mod dto;
pub mod ids;
pub mod log;
pub mod macros;
pub mod perf;
#[cfg(test)]
pub mod test;

pub(crate) mod config;
pub(crate) mod infra;
pub(crate) mod lifecycle;
pub(crate) mod ops;
pub(crate) mod storage;
pub(crate) mod workflow;

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
/// All canister endpoints must convert this into a public error envelope
/// defined in dto/.
///

#[derive(Debug, ThisError)]
pub(crate) enum Error {
    #[error(transparent)]
    Access(#[from] access::AccessError),

    #[error(transparent)]
    Config(#[from] config::ConfigError),

    #[error(transparent)]
    Domain(#[from] domain::DomainError),

    #[error(transparent)]
    Infra(#[from] infra::InfraError),

    #[error(transparent)]
    Ops(#[from] ops::OpsError),

    #[error(transparent)]
    Storage(#[from] storage::StorageError),

    #[error(transparent)]
    Workflow(#[from] workflow::WorkflowError),
}

// init and validate config
// called from here as config is pub(crate)
pub fn init_config(toml: &str) -> Result<(), String> {
    config::Config::init_from_toml(toml)
        .map(|_| ())
        .map_err(|err| err.to_string())
}
