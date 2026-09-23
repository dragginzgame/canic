use crate::{
    dto::template::{
        TemplateChunkSetInfoResponse, TemplateManifestInput, WasmStoreCatalogEntryResponse,
        WasmStoreDeletionCycleReclamationRequest, WasmStoreDeletionCycleReclamationResponse,
        WasmStoreGcTarget, WasmStoreStatusResponse,
    },
    ids::{TemplateId, TemplateVersion},
};
use canic_core::cdk::types::Principal;
use canic_core::control_plane_support::{error::InternalError, ops::cost_guard::CostGuardPermit};

use super::super::WasmStoreInternalClient;

// Fetch the approved live catalog from one Wasm Store.
pub(super) async fn store_catalog(
    store_pid: Principal,
) -> Result<Vec<WasmStoreCatalogEntryResponse>, InternalError> {
    WasmStoreInternalClient::new(store_pid).catalog().await
}

// Fetch deterministic chunk-set metadata for one release from one wasm store.
pub(super) async fn store_chunk_set_info(
    _publication_permit: &CostGuardPermit,
    store_pid: Principal,
    template_id: &TemplateId,
    version: &TemplateVersion,
) -> Result<TemplateChunkSetInfoResponse, InternalError> {
    WasmStoreInternalClient::new(store_pid)
        .info(template_id, version)
        .await
}

// Fetch current occupied-byte and retention state from one wasm store.
pub(super) async fn store_status(
    store_pid: Principal,
) -> Result<WasmStoreStatusResponse, InternalError> {
    WasmStoreInternalClient::new(store_pid).status().await
}

// Stage one approved manifest into one live wasm store.
pub(super) async fn store_stage_manifest(
    publication_permit: &CostGuardPermit,
    store_pid: Principal,
    request: TemplateManifestInput,
) -> Result<(), InternalError> {
    WasmStoreInternalClient::new(store_pid)
        .stage_manifest(publication_permit, request)
        .await
}

// Mark one local wasm store as prepared for store-local GC execution.
pub(super) async fn store_prepare_gc(
    store_pid: Principal,
    operation_id: [u8; 32],
) -> Result<(), InternalError> {
    WasmStoreInternalClient::new(store_pid)
        .run_gc(operation_id, WasmStoreGcTarget::Prepared)
        .await
}

// Mark one local wasm store as actively executing store-local GC.
pub(super) async fn store_begin_gc(
    store_pid: Principal,
    operation_id: [u8; 32],
) -> Result<(), InternalError> {
    WasmStoreInternalClient::new(store_pid)
        .run_gc(operation_id, WasmStoreGcTarget::Complete)
        .await
}

// Mark one local wasm store as having completed the current local GC pass.
pub(super) async fn store_complete_gc(
    store_pid: Principal,
    operation_id: [u8; 32],
) -> Result<(), InternalError> {
    WasmStoreInternalClient::new(store_pid)
        .run_gc(operation_id, WasmStoreGcTarget::Complete)
        .await
}

// Return transferable cycles from one empty GC-complete Store to its authenticated root.
pub(super) async fn store_reclaim_deletion_cycles(
    store_pid: Principal,
    retained_cycles_target: u128,
) -> Result<WasmStoreDeletionCycleReclamationResponse, InternalError> {
    WasmStoreInternalClient::new(store_pid)
        .reclaim_deletion_cycles(WasmStoreDeletionCycleReclamationRequest {
            retained_cycles_target,
        })
        .await
}

// Fetch one deterministic chunk for one release from one wasm store.
pub(super) async fn store_chunk(
    _publication_permit: &CostGuardPermit,
    store_pid: Principal,
    template_id: &TemplateId,
    version: &TemplateVersion,
    chunk_index: u32,
) -> Result<Vec<u8>, InternalError> {
    WasmStoreInternalClient::new(store_pid)
        .chunk(template_id, version, chunk_index)
        .await
}
