//! Control-plane runtime for root and `wasm_store` orchestration.
//!
//! This crate layers the template publication and managed-store workflows on
//! top of `canic-core` and is re-exported through the `canic` facade when the
//! control-plane feature is enabled.

const _: () = {
    #[canic_memory::__reexports::ctor::ctor(
        anonymous,
        crate_path = canic_memory::__reexports::ctor
    )]
    fn __canic_reserve_template_memory_range() {
        canic_memory::ic_memory_range!(10, 12);
    }

    #[canic_memory::__reexports::ctor::ctor(
        anonymous,
        crate_path = canic_memory::__reexports::ctor
    )]
    fn __canic_reserve_control_plane_memory_range() {
        canic_memory::ic_memory_range!(60, 62);
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
