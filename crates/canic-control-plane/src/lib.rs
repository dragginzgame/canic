const _: () = {
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
pub mod ids {
    pub use canic_core::ids::{BuildNetwork, CanisterRole};
    pub use canic_template_types::{
        TemplateChunkKey, TemplateChunkingMode, TemplateId, TemplateManifestState,
        TemplateReleaseKey, TemplateVersion, WasmStoreBinding, WasmStoreGcMode, WasmStoreGcStatus,
    };
}
pub(crate) mod ops;
pub mod runtime;
pub mod schema;
pub(crate) mod storage;
pub(crate) mod support;
pub(crate) mod workflow;
