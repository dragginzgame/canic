use crate::{
    dto::template::{
        TemplateChunkSetInfoResponse, TemplateManifestInput, WasmStoreCatalogEntryResponse,
        WasmStoreStatusResponse,
    },
    ids::{TemplateId, TemplateVersion, WasmStoreBinding},
    ops::storage::{state::subnet::SubnetStateOps, template::TemplateChunkedOps},
};
use canic_core::cdk::types::Principal;
use canic_core::control_plane_support::{error::InternalError, ops::cost_guard::CostGuardPermit};

use super::super::WasmStoreInternalClient;
use super::error::PublicationWorkflowError;

// Fetch the approved embedded catalog from one wasm store.
pub(super) async fn store_catalog(
    _publication_permit: &CostGuardPermit,
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
pub(super) async fn store_prepare_gc(store_pid: Principal) -> Result<(), InternalError> {
    WasmStoreInternalClient::new(store_pid).prepare_gc().await
}

// Mark one local wasm store as actively executing store-local GC.
pub(super) async fn store_begin_gc(store_pid: Principal) -> Result<(), InternalError> {
    WasmStoreInternalClient::new(store_pid).begin_gc().await
}

// Mark one local wasm store as having completed the current local GC pass.
pub(super) async fn store_complete_gc(store_pid: Principal) -> Result<(), InternalError> {
    WasmStoreInternalClient::new(store_pid).complete_gc().await
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

// Resolve the configured logical binding for one registered store canister id.
pub(super) fn store_binding_for_pid(
    store_pid: Principal,
) -> Result<WasmStoreBinding, InternalError> {
    SubnetStateOps::wasm_store_binding_for_pid(store_pid)
        .ok_or_else(|| PublicationWorkflowError::StoreNotRegistered(store_pid).into())
}

// Return deterministic chunk bytes from the current canister's local bootstrap source.
pub(super) fn local_chunk(
    template_id: &TemplateId,
    version: &TemplateVersion,
    chunk_index: u32,
) -> Result<Vec<u8>, InternalError> {
    let response = TemplateChunkedOps::chunk_response(template_id, version, chunk_index)?;
    Ok(response.bytes)
}
