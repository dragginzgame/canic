//! Core Canic library used inside canisters.
//!
//! Most users should depend on the `canic` facade crate, which re-exports this crate
//! under `canic::core` and exposes the common entrypoint macros:
//! - `canic::build!` (in `build.rs`) to validate/embed `canic.toml`
//! - `canic::start!` (in `lib.rs`) to wire lifecycle hooks and export endpoints
//!
//! ## Layering
//!
//! Canic is organized to keep endpoint code thin and coordination centralized:
//! - `access/` contains access expressions, predicates, and metrics for boundary enforcement.
//! - `workflow/` implements orchestration and lifecycle workflows.
//! - `domain/` contains pure value and decision helpers.
//! - `model/` contains pure runtime state models shared by ops and storage.
//! - `ops/` provides mechanical, reusable side-effecting operations.
//! - `storage/` owns stable-memory-backed schemas and helpers.
//! - `view/` exposes internal read-only projections over stored/runtime state.
//! - macro entrypoints live in the `canic` facade crate.
//!
//! The default flow is: endpoints → workflow → domain/decision helpers → ops → storage/model.

#[doc(hidden)]
pub mod access;
pub mod api;
#[doc(hidden)]
pub mod bootstrap;
pub mod cdk;
#[doc(hidden)]
pub mod control_plane_support;
#[doc(hidden)]
pub mod dispatch;
pub mod dto;
#[doc(hidden)]
pub mod error;
mod format;
pub mod ids;
#[doc(hidden)]
pub mod ingress;
pub mod log;
pub mod memory;
mod memory_macros;
pub mod perf;
pub mod protocol;
pub mod replay_policy;
#[doc(hidden)]
pub mod role_contract;
#[doc(hidden)]
pub mod shared_support;
#[doc(hidden)]
pub mod state_contract;
#[cfg(test)]
pub mod test;

pub(crate) mod config;
pub(crate) mod domain;
pub(crate) mod infra;
pub(crate) mod lifecycle;
pub(crate) mod model;
pub(crate) mod ops;
pub(crate) mod storage;
pub(crate) mod view;
pub(crate) mod workflow;

pub(crate) use error::{InternalError, InternalErrorClass, InternalErrorOrigin};

/// Internal re-exports required for macro expansion.
/// Not part of the public API.
#[doc(hidden)]
pub mod __reexports {
    pub use ::ic_memory;
    pub use ::ic_memory::__reexports::ctor;
}

///
/// Consts
///

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CANIC_MEMORY_MIN: u8 = role_contract::allocation::CANIC_CORE_MIN_ID;
pub const CANIC_MEMORY_MAX: u8 = role_contract::allocation::CANIC_CORE_MAX_ID;
// Canonical hardcoded 1 MiB chunk size for Canic wasm staging/install flows.
// The management canister wasm chunk store rejects larger payloads.
pub const CANIC_WASM_CHUNK_BYTES: usize = 1_048_576;

ic_memory::ic_memory_range!(
    start = role_contract::allocation::CANIC_CORE_MIN_ID,
    end = role_contract::allocation::CANIC_CORE_MAX_ID,
);

#[cfg(test)]
const _: () = {
    fn __canic_memory_test_bootstrap() {
        crate::api::runtime::MemoryRuntimeApi::bootstrap_registry()
            .expect("test stable-memory bootstrap");
    }

    #[crate::__reexports::ctor::ctor(
        unsafe,
        anonymous,
        crate_path = crate::__reexports::ctor
    )]
    fn __canic_install_memory_test_bootstrap_hook() {
        crate::memory::runtime::install_test_bootstrap_hook(__canic_memory_test_bootstrap);
    }
};

#[macro_export]
macro_rules! perf {
    ($($label:tt)*) => {{
        $crate::perf::PERF_LAST.with(|last| {
            let now = $crate::perf::perf_counter();
            let then = *last.borrow();
            let delta = now.saturating_sub(then);

            *last.borrow_mut() = now;

            let label = format!($($label)*);
            $crate::perf::record_checkpoint(module_path!(), &label, delta);
        });
    }};
}

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

#[cfg(test)]
mod memory_bootstrap_tests {
    #[test]
    fn installs_host_test_bootstrap_hook() {
        assert!(crate::memory::runtime::has_test_bootstrap_hook());
    }
}
