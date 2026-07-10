//! Control-plane runtime for root and `wasm_store` orchestration.
//!
//! This crate layers the template publication and managed-store workflows on
//! top of `canic-core` and is re-exported through the `canic` facade when the
//! control-plane feature is enabled.

canic_core::ic_memory_range!(
    start = canic_core::role_contract::allocation::CANIC_CONTROL_PLANE_MIN_ID,
    end = canic_core::role_contract::allocation::CANIC_CONTROL_PLANE_MAX_ID,
);

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
pub(crate) mod config;
pub mod dto;
pub mod ids;
pub(crate) mod ops;
#[cfg(feature = "root-control-plane")]
pub mod runtime;
pub mod schema;
pub mod state_contract;
pub(crate) mod storage;
#[cfg(feature = "root-control-plane")]
pub(crate) mod workflow;
