use crate::dto::prelude::*;
use crate::ids::{
    TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    WasmStoreGcMode,
};

///
/// TemplateManifestInput
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
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

///
/// TemplateManifestResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
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

///
/// TemplateChunkSetInput
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TemplateChunkSetInput {
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub payload_hash: Vec<u8>,
    pub payload_size_bytes: u64,
    pub chunks: Vec<Vec<u8>>,
}

///
/// TemplateChunkSetPrepareInput
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TemplateChunkSetPrepareInput {
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub payload_hash: Vec<u8>,
    pub payload_size_bytes: u64,
    pub chunk_hashes: Vec<Vec<u8>>,
}

///
/// TemplateChunkInput
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TemplateChunkInput {
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub chunk_index: u32,
    pub bytes: Vec<u8>,
}

///
/// TemplateChunkSetInfoResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TemplateChunkSetInfoResponse {
    pub chunk_hashes: Vec<Vec<u8>>,
}

///
/// TemplateChunkResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TemplateChunkResponse {
    pub bytes: Vec<u8>,
}

///
/// WasmStoreCatalogEntryResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreCatalogEntryResponse {
    pub role: CanisterRole,
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub payload_hash: Vec<u8>,
    pub payload_size_bytes: u64,
}

///
/// WasmStoreTemplateStatusResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreTemplateStatusResponse {
    pub template_id: TemplateId,
    pub versions: u16,
}

///
/// WasmStoreGcStatusResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreGcStatusResponse {
    pub mode: WasmStoreGcMode,
    pub changed_at: u64,
    pub prepared_at: Option<u64>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub runs_completed: u32,
}

///
/// WasmStoreStatusResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreStatusResponse {
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

///
/// WasmStorePublicationSlotResponse
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum WasmStorePublicationSlotResponse {
    Active,
    Detached,
    Retired,
}

///
/// WasmStoreOverviewStoreResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreOverviewStoreResponse {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
    pub created_at: u64,
    pub publication_slot: Option<WasmStorePublicationSlotResponse>,
    pub gc: WasmStoreGcStatusResponse,
    pub payload_bytes: u64,
    pub payload_size: String,
    pub max_store_bytes: u64,
    pub max_store_size: String,
    pub remaining_payload_bytes: u64,
    pub remaining_payload_size: String,
    pub headroom_bytes: Option<u64>,
    pub headroom_size: Option<String>,
    pub within_headroom: bool,
    pub template_count: u32,
    pub max_templates: Option<u32>,
    pub release_count: u32,
    pub max_template_versions_per_template: Option<u16>,
    pub templates: Vec<WasmStoreTemplateStatusResponse>,
}

///
/// WasmStoreOverviewResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreOverviewResponse {
    pub publication: WasmStorePublicationStateResponse,
    pub stores: Vec<WasmStoreOverviewStoreResponse>,
}

///
/// TemplateStagingStatusResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TemplateStagingStatusResponse {
    pub role: CanisterRole,
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub store_binding: WasmStoreBinding,
    pub chunking_mode: TemplateChunkingMode,
    pub payload_size_bytes: u64,
    pub payload_size: String,
    pub chunk_set_present: bool,
    pub expected_chunk_count: u32,
    pub stored_chunk_count: u32,
    pub publishable: bool,
}

///
/// WasmStoreBootstrapDebugResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreBootstrapDebugResponse {
    pub ready_for_bootstrap: bool,
    pub bootstrap: Option<TemplateStagingStatusResponse>,
    pub staged: Vec<TemplateStagingStatusResponse>,
}

///
/// WasmStorePublicationStateResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStorePublicationStateResponse {
    pub active_binding: Option<WasmStoreBinding>,
    pub detached_binding: Option<WasmStoreBinding>,
    pub retired_binding: Option<WasmStoreBinding>,
    pub generation: u64,
    pub changed_at: u64,
    pub retired_at: u64,
}

///
/// WasmStorePublicationFinalizationStatusResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStorePublicationFinalizationStatusResponse {
    pub finalized_binding: Option<WasmStoreBinding>,
    pub finalized_at: u64,
}

///
/// WasmStoreRetiredStoreStatusResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreRetiredStoreStatusResponse {
    pub retired_binding: WasmStoreBinding,
    pub generation: u64,
    pub retired_at: u64,
    pub gc_ready: bool,
    pub reclaimable_store_bytes: u64,
    pub store: WasmStoreStatusResponse,
}

///
/// WasmStoreFinalizedStoreResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreFinalizedStoreResponse {
    pub binding: WasmStoreBinding,
    pub store_pid: Principal,
}

///
/// WasmStoreAdminCommand
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum WasmStoreAdminCommand {
    PublishCurrentReleaseToStore {
        store_pid: Principal,
    },
    PublishCurrentReleaseToCurrentStore,
    SetPublicationBinding {
        binding: WasmStoreBinding,
    },
    ClearPublicationBinding,
    RetireDetachedBinding,
    PrepareRetiredStoreGc,
    BeginRetiredStoreGc,
    CompleteRetiredStoreGc,
    FinalizeRetiredBinding,
    DeleteFinalizedStore {
        binding: WasmStoreBinding,
        store_pid: Principal,
    },
}

///
/// WasmStoreAdminResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum WasmStoreAdminResponse {
    PublishedCurrentReleaseToStore {
        store_pid: Principal,
    },
    PublishedCurrentReleaseToCurrentStore,
    SetPublicationBinding {
        binding: WasmStoreBinding,
    },
    ClearedPublicationBinding,
    RetiredDetachedBinding {
        binding: Option<WasmStoreBinding>,
    },
    PreparedRetiredStoreGc {
        binding: Option<WasmStoreBinding>,
    },
    BeganRetiredStoreGc {
        binding: Option<WasmStoreBinding>,
    },
    CompletedRetiredStoreGc {
        binding: Option<WasmStoreBinding>,
    },
    FinalizedRetiredBinding {
        result: Option<WasmStoreFinalizedStoreResponse>,
    },
    DeletedFinalizedStore {
        binding: WasmStoreBinding,
        store_pid: Principal,
    },
}
