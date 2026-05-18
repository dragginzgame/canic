//! Control-plane runtime for root and `wasm_store` orchestration.
//!
//! This crate layers the template publication and managed-store workflows on
//! top of `canic-core` and is re-exported through the `canic` facade when the
//! control-plane feature is enabled.

const _: () = {
    #[canic_memory::__reexports::ctor::ctor(
        unsafe,
        anonymous,
        crate_path = canic_memory::__reexports::ctor
    )]
    fn __canic_reserve_control_plane_memory_range() {
        canic_memory::ic_memory_range!(80, 85);
    }
};

#[cfg(test)]
const _: () = {
    use std::sync::Once;

    fn __canic_memory_test_bootstrap() {
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            canic_core::api::runtime::MemoryRuntimeApi::bootstrap_registry()
                .expect("test stable-memory bootstrap");
        });
    }

    #[canic_memory::__reexports::ctor::ctor(
        unsafe,
        anonymous,
        crate_path = canic_memory::__reexports::ctor
    )]
    fn __canic_install_memory_test_bootstrap_hook() {
        canic_memory::runtime::install_test_bootstrap_hook(__canic_memory_test_bootstrap);
    }
};

pub mod api;
pub(crate) mod config;
pub mod dto;
pub mod ids;
pub(crate) mod ops;
pub mod runtime;
pub mod schema;
pub(crate) mod storage;
pub(crate) mod support;
pub(crate) mod workflow;
