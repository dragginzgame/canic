//
// TemplateManifestInput
//

use crate::ids::{
    CanisterRole, TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion,
    WasmStoreBinding, WasmStoreGcMode,
};
use ic_platform::principal::ExternalPrincipal as Principal;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TemplateManifestInput {
    pub template_id: TemplateId,
    pub role: CanisterRole,
    pub version: TemplateVersion,
    pub payload_hash: Vec<u8>,
    pub payload_size_bytes: u64,
    pub store_binding: WasmStoreBinding,
    pub chunking_mode: TemplateChunkingMode,
    pub manifest_state: TemplateManifestState,
    pub approved_at: Option<u64>,
    pub created_at: u64,
}

//
// TemplateManifestResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TemplateManifestResponse {
    pub template_id: TemplateId,
    pub role: CanisterRole,
    pub version: TemplateVersion,
    pub payload_hash: Vec<u8>,
    pub payload_size_bytes: u64,
    pub store_binding: WasmStoreBinding,
    pub chunking_mode: TemplateChunkingMode,
    pub manifest_state: TemplateManifestState,
    pub approved_at: Option<u64>,
    pub created_at: u64,
}

//
// TemplateChunkSetPrepareInput
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TemplateChunkSetPrepareInput {
    /// Optional approved manifest admitted atomically with this exact chunk set.
    pub manifest: Option<TemplateManifestInput>,
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub payload_hash: Vec<u8>,
    pub payload_size_bytes: u64,
    pub chunk_hashes: Vec<Vec<u8>>,
}

//
// TemplateChunkInput
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct TemplateChunkInput {
    /// Exact metadata admitted with chunk zero; absent for subsequent chunks.
    pub preparation: Option<TemplateChunkSetPrepareInput>,
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub chunk_index: u32,
    pub bytes: Vec<u8>,
}

//
// TemplateChunkSetInfoResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TemplateChunkSetInfoResponse {
    pub chunk_hashes: Vec<Vec<u8>>,
}

//
// TemplateChunkResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TemplateChunkResponse {
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}

/// Exact template release key used by Store command inspection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TemplateLookupRequest {
    pub template_id: TemplateId,
    pub version: TemplateVersion,
}

/// Exact template chunk key used by the Store's bounded read lane.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TemplateChunkRequest {
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub chunk_index: u32,
}

//
// WasmStoreCatalogEntryResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct WasmStoreCatalogEntryResponse {
    pub role: CanisterRole,
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub payload_hash: Vec<u8>,
    pub payload_size_bytes: u64,
}

//
// WasmStoreTemplateStatusResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct WasmStoreTemplateStatusResponse {
    pub template_id: TemplateId,
    pub versions: u16,
}

//
// WasmStoreGcStatusResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct WasmStoreGcStatusResponse {
    pub mode: WasmStoreGcMode,
    /// Whole seconds since the Unix epoch; zero means no transition yet.
    pub changed_at: u64,
    /// Whole seconds since the Unix epoch, absent until GC is prepared.
    pub prepared_at: Option<u64>,
    /// Whole seconds since the Unix epoch, absent until GC starts.
    pub started_at: Option<u64>,
    /// Whole seconds since the Unix epoch, absent until GC completes.
    pub completed_at: Option<u64>,
    pub runs_completed: u32,
}

//
// WasmStoreStatusResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct WasmStoreStatusResponse {
    pub inventory: WasmStoreInventoryResponse,
    pub gc: WasmStoreGcStatusResponse,
    pub occupied_store_bytes: u64,
    pub occupied_store_size: String,
    pub max_store_bytes: u64,
    pub max_store_size: String,
    pub remaining_store_bytes: u64,
    pub remaining_store_size: String,
    pub headroom_bytes: Option<u64>,
    pub headroom_size: Option<String>,
    pub within_headroom: bool,
    pub template_count: u32,
    pub max_templates: Option<u32>,
    pub release_count: u32,
    pub max_template_versions_per_template: Option<u16>,
    pub templates: Vec<WasmStoreTemplateStatusResponse>,
}

/// Aggregate retained inventory; template and opaque fixture chunks stay distinct.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct WasmStoreInventoryResponse {
    pub approved_catalog_entries: u64,
    pub expected_template_chunks: u64,
    pub stored_template_chunks: u64,
    pub fixture_sources: u64,
    pub expected_fixture_chunks: u64,
    pub stored_fixture_chunks: u64,
}

/// Store garbage-collection detail projected through the operation lane.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct WasmStoreGcOperationStatus {
    pub operation_id: [u8; 32],
    pub gc: WasmStoreGcStatusResponse,
}

/// Exact outcome requested from the existing Store retirement operation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum WasmStoreGcTarget {
    Complete,
    Prepared,
}

/// Root-owned retirement intent; preparation replay never authorizes collection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct WasmStoreGcRequest {
    pub operation_id: [u8; 32],
    pub target: WasmStoreGcTarget,
}

/// Minimum operational headroom retained above the live freezing reserve while
/// an empty Store returns cycles and remains available for stop/delete calls.
pub const WASM_STORE_DELETION_EXECUTION_RESERVE_CYCLES: u128 = 300_000_000_000;

/// Headroom below the retained target used to absorb post-call cycle refunds.
pub const WASM_STORE_DELETION_CALL_REFUND_HEADROOM_CYCLES: u128 = 150_000_000_000;

//
// WasmStoreDeletionCycleReclamationRequest
//

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct WasmStoreDeletionCycleReclamationRequest {
    pub retained_cycles_target: u128,
}

//
// WasmStoreDeletionCycleReclamationResponse
//

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct WasmStoreDeletionCycleReclamationResponse {
    pub destination: Principal,
    pub cycles_before: u128,
    pub retained_cycles_target: u128,
    pub cycles_transferred: u128,
    pub cycles_after: u128,
}

//
// WasmStorePublicationSlotResponse
//

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum WasmStorePublicationSlotResponse {
    Active,
    Detached,
    Retired,
}

//
// WasmStoreOverviewStoreResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreOverviewStoreResponse {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
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

//
// WasmStoreOverviewResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreOverviewResponse {
    pub publication: WasmStorePublicationStateResponse,
    pub stores: Vec<WasmStoreOverviewStoreResponse>,
}

//
// TemplateStagingStatusResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TemplateStagingStatusResponse {
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub manifest: Option<TemplateManifestResponse>,
    pub chunk_set_present: bool,
    pub expected_chunk_count: u32,
    pub expected_chunk_hashes: Vec<Vec<u8>>,
    pub payload_hash: Option<Vec<u8>>,
    pub payload_size_bytes: Option<u64>,
    pub stored_chunk_hashes: Vec<Option<Vec<u8>>>,
    pub stored_chunk_count: u32,
    pub complete: bool,
}

//
// WasmStorePublicationStateResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct WasmStorePublicationStateResponse {
    pub active_binding: Option<WasmStoreBinding>,
    pub detached_binding: Option<WasmStoreBinding>,
    pub retired_binding: Option<WasmStoreBinding>,
    pub generation: u64,
    /// Whole seconds since the Unix epoch; zero means no transition yet.
    pub changed_at: u64,
    /// Whole seconds since the Unix epoch; zero means no retired binding.
    pub retired_at: u64,
}

//
// WasmStoreAdminCommand
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub enum WasmStoreAdminCommand {
    PublishActiveReleaseSet,
}

//
// WasmStoreAdminResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub enum WasmStoreAdminResponse {
    PublishedActiveReleaseSet,
}
