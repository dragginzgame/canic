use crate::ids::{
    CanisterRole, TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion,
    WasmStoreBinding, WasmStoreGcMode,
};
use candid::{CandidType, Principal};
use canic_core::{
    dto::{
        capability::{NonrootCyclesCapabilityEnvelopeV1, NonrootCyclesCapabilityResponseV1},
        cascade::{StateSnapshotInput, TopologySnapshotInput},
        cycles::CycleTrackerEntry,
        fleet_activation::{
            FleetActivationRequest, FleetActivationStatusResponse, FleetCredentialGenerationRequest,
        },
        page::{Page, PageRequest},
        role::{
            CycleBalanceStatusResponse, OperationReceipt, OperationStatusRequest,
            RoleOverviewResponse,
        },
    },
    ids::FleetSubnetWasmStoreAuthority,
};
use serde::Deserialize;

//
// TemplateManifestInput
//

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

//
// TemplateManifestResponse
//

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

//
// TemplateChunkSetPrepareInput
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TemplateChunkSetPrepareInput {
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub payload_hash: Vec<u8>,
    pub payload_size_bytes: u64,
    pub chunk_hashes: Vec<Vec<u8>>,
}

//
// TemplateChunkInput
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TemplateChunkInput {
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub chunk_index: u32,
    pub bytes: Vec<u8>,
}

//
// TemplateChunkSetInfoResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TemplateChunkSetInfoResponse {
    pub chunk_hashes: Vec<Vec<u8>>,
}

//
// TemplateChunkResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TemplateChunkResponse {
    pub bytes: Vec<u8>,
}

/// Exact template release key used by Store command inspection.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TemplateLookupRequest {
    pub template_id: TemplateId,
    pub version: TemplateVersion,
}

/// Exact template chunk key used by the Store's bounded read lane.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TemplateChunkRequest {
    pub template_id: TemplateId,
    pub version: TemplateVersion,
    pub chunk_index: u32,
}

//
// WasmStoreCatalogEntryResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
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

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreTemplateStatusResponse {
    pub template_id: TemplateId,
    pub versions: u16,
}

//
// WasmStoreGcStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreGcStatusResponse {
    pub mode: WasmStoreGcMode,
    pub changed_at: u64,
    pub prepared_at: Option<u64>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub runs_completed: u32,
}

//
// WasmStoreStatusResponse
//

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

/// Store garbage-collection detail projected through the operation lane.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreGcOperationStatus {
    pub operation_id: [u8; 32],
    pub gc: WasmStoreGcStatusResponse,
}

/// Closed Store control-plane command union.
#[derive(CandidType, Deserialize)]
pub enum StoreCommand {
    ActivateFleet(FleetActivationRequest),
    InspectTemplate(TemplateLookupRequest),
    PrepareChunkSet(TemplateChunkSetPrepareInput),
    PrepareFleetCredential(FleetCredentialGenerationRequest),
    ReclaimDeletionCycles(WasmStoreDeletionCycleReclamationRequest),
    RespondCapability(NonrootCyclesCapabilityEnvelopeV1),
    RunGc(OperationStatusRequest),
    StageManifest(TemplateManifestInput),
    SynchronizeState(StateSnapshotInput),
    SynchronizeTopology(TopologySnapshotInput),
}

/// Closed response union correlated to one accepted Store command.
#[derive(CandidType, Deserialize)]
pub enum StoreCommandResponse {
    InspectTemplate(TemplateChunkSetInfoResponse),
    OperationAccepted(OperationReceipt),
    PrepareChunkSet(TemplateChunkSetInfoResponse),
    ReclaimDeletionCycles(WasmStoreDeletionCycleReclamationResponse),
    RespondCapability(NonrootCyclesCapabilityResponseV1),
    StageManifest,
    SynchronizeState,
    SynchronizeTopology,
}

/// Closed Store observation selector carried by its single status query.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum StoreStatusRequest {
    Authority,
    Catalog,
    CycleBalance,
    CycleHistory(PageRequest),
    Operation(OperationStatusRequest),
    Overview,
    Storage,
}

/// Store-owned durable operation detail selected by one operation ID.
#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the accepted Candid union carries each existing status DTO directly"
)]
pub enum StoreOperationStatusResponse {
    FleetActivation(FleetActivationStatusResponse),
    GarbageCollection(WasmStoreGcOperationStatus),
}

/// Closed response union for the Store's single status query.
#[derive(CandidType, Deserialize)]
pub enum StoreStatusResponse {
    Authority(FleetSubnetWasmStoreAuthority),
    Catalog(Vec<WasmStoreCatalogEntryResponse>),
    CycleBalance(CycleBalanceStatusResponse),
    CycleHistory(Page<CycleTrackerEntry>),
    Operation(StoreOperationStatusResponse),
    Overview(RoleOverviewResponse),
    Storage(WasmStoreStatusResponse),
}

/// Minimum operational headroom retained above the live freezing reserve while
/// an empty Store returns cycles and remains available for stop/delete calls.
pub const WASM_STORE_DELETION_EXECUTION_RESERVE_CYCLES: u128 = 300_000_000_000;

/// Headroom below the retained target used to absorb post-call cycle refunds.
pub const WASM_STORE_DELETION_CALL_REFUND_HEADROOM_CYCLES: u128 = 150_000_000_000;

//
// WasmStoreDeletionCycleReclamationRequest
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreDeletionCycleReclamationRequest {
    pub retained_cycles_target: u128,
}

//
// WasmStoreDeletionCycleReclamationResponse
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
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

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum WasmStorePublicationSlotResponse {
    Active,
    Detached,
    Retired,
}

//
// WasmStoreOverviewStoreResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreOverviewStoreResponse {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
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

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreOverviewResponse {
    pub publication: WasmStorePublicationStateResponse,
    pub stores: Vec<WasmStoreOverviewStoreResponse>,
}

//
// TemplateStagingStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[cfg(test)]
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

//
// WasmStorePublicationStateResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStorePublicationStateResponse {
    pub active_binding: Option<WasmStoreBinding>,
    pub detached_binding: Option<WasmStoreBinding>,
    pub retired_binding: Option<WasmStoreBinding>,
    pub generation: u64,
    pub changed_at: u64,
    pub retired_at: u64,
}

//
// WasmStoreAdminCommand
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum WasmStoreAdminCommand {
    PublishActiveReleaseSet,
}

//
// WasmStoreAdminResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum WasmStoreAdminResponse {
    PublishedActiveReleaseSet,
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Decode, Encode};

    #[test]
    fn store_status_request_keeps_the_manifest_exact_flat_variants() {
        let page = PageRequest {
            limit: 10,
            offset: 0,
        };
        let requests = [
            StoreStatusRequest::Authority,
            StoreStatusRequest::Catalog,
            StoreStatusRequest::CycleBalance,
            StoreStatusRequest::CycleHistory(page),
            StoreStatusRequest::Operation(OperationStatusRequest {
                operation_id: [5; 32],
            }),
            StoreStatusRequest::Overview,
            StoreStatusRequest::Storage,
        ];

        for request in requests {
            let bytes = Encode!(&request).expect("encode Store status request");
            assert_eq!(
                Decode!(&bytes, StoreStatusRequest).expect("decode Store status request"),
                request
            );
        }
    }
}
