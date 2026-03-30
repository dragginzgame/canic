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
        canic_memory::ic_memory_range!(60, 60);
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
