//! Native Root Store inventory; storage retains its canonical principal representation.
use crate::{
    dto::template::{
        WasmStoreGcStatusResponse, WasmStorePublicationSlotResponse,
        WasmStorePublicationStateResponse, WasmStoreTemplateStatusResponse,
    },
    ids::WasmStoreBinding,
};
/// One complete registered Store, including publication and collection evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Store {
    pub binding: WasmStoreBinding,
    pub pid: ic_platform::Principal,
    /// Whole seconds since the Unix epoch, matching publication and GC timestamps.
    pub created_at: u64,
    pub publication_slot: Option<WasmStorePublicationSlotResponse>,
    pub gc: WasmStoreGcStatusResponse,
    pub approved_payload_bytes: u64,
    pub approved_payload_size: String,
    pub max_store_bytes: u64,
    pub max_store_size: String,
    pub remaining_approved_payload_bytes: u64,
    pub remaining_approved_payload_size: String,
    pub headroom_bytes: Option<u64>,
    pub headroom_size: Option<String>,
    pub within_approved_headroom: bool,
    pub approved_template_count: u32,
    pub max_templates: Option<u32>,
    pub approved_release_count: u32,
    pub max_template_versions_per_template: Option<u16>,
    pub approved_templates: Vec<WasmStoreTemplateStatusResponse>,
}

/// Complete Root publication state and registered Store inventory.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Overview {
    pub publication: WasmStorePublicationStateResponse,
    pub stores: Vec<Store>,
}
