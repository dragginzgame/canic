//! Shared template and wasm-store identifier/value types.

pub mod dto;
pub mod ids;

pub use ids::{
    CanisterRole, TemplateChunkKey, TemplateChunkingMode, TemplateId, TemplateManifestState,
    TemplateReleaseKey, TemplateVersion, WasmStoreBinding, WasmStoreGcMode, WasmStoreGcStatus,
};
