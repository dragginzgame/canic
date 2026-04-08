use crate::{
    dto::template::{
        TemplateChunkResponse, TemplateChunkSetInfoResponse, TemplateManifestInput,
        WasmStoreCatalogEntryResponse, WasmStoreStatusResponse,
    },
    ids::{TemplateId, TemplateVersion, WasmStoreBinding},
    ops::storage::{state::subnet::SubnetStateOps, template::TemplateChunkedOps},
};
use candid::CandidType;
use canic_core::__control_plane_core as cp_core;
use cp_core::{InternalError, InternalErrorOrigin, cdk::types::Principal, protocol};

use super::super::call_store_result;

// Borrowed chunk publish input for store-side chunk staging.
#[derive(CandidType)]
pub(super) struct TemplateChunkInputRef<'a> {
    pub template_id: &'a TemplateId,
    pub version: &'a TemplateVersion,
    pub chunk_index: u32,
    pub bytes: &'a [u8],
}

// Fetch the approved embedded catalog from one wasm store.
pub(super) async fn store_catalog(
    store_pid: Principal,
) -> Result<Vec<WasmStoreCatalogEntryResponse>, InternalError> {
    call_store_result(store_pid, protocol::CANIC_WASM_STORE_CATALOG, ()).await
}

// Fetch deterministic chunk-set metadata for one release from one wasm store.
pub(super) async fn store_chunk_set_info(
    store_pid: Principal,
    template_id: &TemplateId,
    version: &TemplateVersion,
) -> Result<TemplateChunkSetInfoResponse, InternalError> {
    call_store_result(
        store_pid,
        protocol::CANIC_WASM_STORE_INFO,
        (
            template_id.as_str().to_string(),
            version.as_str().to_string(),
        ),
    )
    .await
}

// Fetch current occupied-byte and retention state from one wasm store.
pub(super) async fn store_status(
    store_pid: Principal,
) -> Result<WasmStoreStatusResponse, InternalError> {
    call_store_result(store_pid, protocol::CANIC_WASM_STORE_STATUS, ()).await
}

// Stage one approved manifest into one live wasm store.
pub(super) async fn store_stage_manifest(
    store_pid: Principal,
    request: TemplateManifestInput,
) -> Result<(), InternalError> {
    call_store_result(
        store_pid,
        protocol::CANIC_WASM_STORE_STAGE_MANIFEST,
        (request,),
    )
    .await
}

// Mark one local wasm store as prepared for store-local GC execution.
pub(super) async fn store_prepare_gc(store_pid: Principal) -> Result<(), InternalError> {
    call_store_result(store_pid, protocol::CANIC_WASM_STORE_PREPARE_GC, ()).await
}

// Mark one local wasm store as actively executing store-local GC.
pub(super) async fn store_begin_gc(store_pid: Principal) -> Result<(), InternalError> {
    call_store_result(store_pid, protocol::CANIC_WASM_STORE_BEGIN_GC, ()).await
}

// Mark one local wasm store as having completed the current local GC pass.
pub(super) async fn store_complete_gc(store_pid: Principal) -> Result<(), InternalError> {
    call_store_result(store_pid, protocol::CANIC_WASM_STORE_COMPLETE_GC, ()).await
}

// Fetch one deterministic chunk for one release from one wasm store.
pub(super) async fn store_chunk(
    store_pid: Principal,
    template_id: &TemplateId,
    version: &TemplateVersion,
    chunk_index: u32,
) -> Result<Vec<u8>, InternalError> {
    let response: TemplateChunkResponse = call_store_result(
        store_pid,
        protocol::CANIC_WASM_STORE_CHUNK,
        (
            template_id.as_str().to_string(),
            version.as_str().to_string(),
            chunk_index,
        ),
    )
    .await?;

    Ok(response.bytes)
}

// Resolve the configured logical binding for one registered store canister id.
pub(super) fn store_binding_for_pid(
    store_pid: Principal,
) -> Result<WasmStoreBinding, InternalError> {
    SubnetStateOps::wasm_store_binding_for_pid(store_pid).ok_or_else(|| {
        InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("wasm store {store_pid} is not registered"),
        )
    })
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
