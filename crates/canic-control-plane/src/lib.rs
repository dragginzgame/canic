//! Control-plane runtime for Fleet Coordinator, root, and `wasm_store` orchestration.
//!
//! This crate layers the Coordinator Registry plus template publication and
//! managed-store workflows on top of `canic-core`. The `canic` facade
//! re-exports each role-specific surface through its matching feature.

use canic_contracts::{dto, ids};

#[cfg(test)]
const _: () = {
    fn __canic_memory_test_bootstrap() {
        canic_core::api::runtime::MemoryRuntimeApi::bootstrap_registry()
            .expect("test stable-memory bootstrap");
    }

    #[canic_core::__reexports::ctor::ctor(
        unsafe,
        anonymous,
        crate_path = canic_core::__reexports::ctor
    )]
    fn __canic_install_memory_test_bootstrap_hook() {
        canic_core::memory::runtime::install_test_bootstrap_hook(__canic_memory_test_bootstrap);
    }
};

pub mod api;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub(crate) mod config;
pub mod installation;

pub(crate) mod ops;
#[cfg(feature = "root-control-plane")]
pub(crate) mod runtime;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub(crate) mod schema;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod state_contract;
#[cfg(any(
    feature = "fleet-coordinator-canister",
    feature = "root-control-plane",
    feature = "wasm-store-canister"
))]
pub(crate) mod storage;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(any(
    feature = "fleet-coordinator-canister",
    feature = "root-control-plane",
    feature = "wasm-store-canister"
))]
pub(crate) mod view;
#[cfg(any(
    feature = "fleet-coordinator-canister",
    feature = "root-control-plane",
    feature = "wasm-store-canister"
))]
pub(crate) mod workflow;
