pub mod chunked;
pub mod gc;
pub mod manifest;

#[cfg(any(test, feature = "wasm-store-canister"))]
pub use chunked::TemplateChunkRecord;
#[cfg(feature = "wasm-store-canister")]
pub use chunked::TemplateChunkSetEntryRecord;
pub use chunked::{TemplateChunkSetRecord, TemplateChunkSetsData};
#[cfg(any(test, feature = "wasm-store-canister"))]
pub use chunked::{TemplateChunkSetStateStore, TemplateChunkStore};
#[cfg(feature = "wasm-store-canister")]
pub use gc::WasmStoreGcStateStore;
pub use gc::{WasmStoreGcStateData, WasmStoreGcStateRecord};
pub use manifest::{
    TemplateManifestEntryRecord, TemplateManifestRecord, TemplateManifestStateStore,
    TemplateManifestsData,
};
