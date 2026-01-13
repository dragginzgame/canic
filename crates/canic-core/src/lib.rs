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
//! - macro entrypoints live in the `canic` facade crate.
//!
//! The default flow is: endpoints → workflow → policy → ops → model.

// -----------------------------------------------------------------------------
// Phase 0: path coherence re-exports (no behavior change)
// -----------------------------------------------------------------------------

pub mod access; // todo - potentially could be pub(crate) but custom errors would have to change
pub mod api;
#[doc(hidden)]
pub mod dispatch;
pub mod domain;
pub mod dto;
pub mod error;
pub mod ids;
pub mod log;
pub mod perf;
pub mod protocol;
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
};

pub(crate) use error::{InternalError, InternalErrorClass, InternalErrorOrigin};

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

// init and validate config
// called from here as config is pub(crate)
pub fn init_config(toml: &str) -> Result<(), String> {
    config::Config::init_from_toml(toml)
        .map(|_| ())
        .map_err(|err| err.to_string())
}
