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
//! - `access/` contains access expressions, predicates, and metrics for boundary enforcement.
//! - `workflow/` implements orchestration and lifecycle workflows.
//! - `policy/` owns deterministic decision rules.
//! - `ops/` provides mechanical, reusable side-effecting operations.
//! - `model/` owns storage (stable memory) and in-process registries/caches.
//! - macro entrypoints live in the `canic` facade crate.
//!
//! The default flow is: endpoints → workflow → policy → ops → model.

pub mod access;
pub mod api;
pub mod bootstrap;
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
pub(crate) mod view;
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
/// Consts
///

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CANIC_MEMORY_MIN: u8 = storage::stable::CANIC_MEMORY_MIN;
pub const CANIC_MEMORY_MAX: u8 = storage::stable::CANIC_MEMORY_MAX;

#[cfg(test)]
#[macro_export]
macro_rules! assert_err_variant {
    ($err:expr, $pat:pat $(if $guard:expr)? $(,)?) => {{
        match $err {
            $pat $(if $guard)? => {}
            other => panic!("unexpected error variant: {other:?}"),
        }
    }};
}
