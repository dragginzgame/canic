//!
//! Subnet-local wasm store canister for approved template chunk sets.
//! Lives in `crates/canisters` as the storage-only `0.18` wasm store role.
//!

#![allow(clippy::unused_async)]

mod gc_state;

use canic::{Error, api::canister::CanisterRole, prelude::*};
use canic::{
    api::canister::template::WasmStoreApi,
    dto::template::{
        TemplateChunkInput, TemplateChunkResponse, TemplateChunkSetInfoResponse,
        TemplateChunkSetPrepareInput, WasmStoreCatalogEntryResponse, WasmStoreStatusResponse,
    },
    ids::{TemplateId, TemplateVersion, WasmStoreGcMode},
};
//
// CANIC
//

canic::start!(CanisterRole::WASM_STORE);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

//
// ENDPOINTS
//

/// canic_wasm_store_catalog
/// Return the approved release catalog for this local wasm store.
#[canic_query(internal, requires(caller::is_root()))]
async fn canic_wasm_store_catalog() -> Result<Vec<WasmStoreCatalogEntryResponse>, Error> {
    WasmStoreApi::template_catalog()
}

/// canic_wasm_store_prepare
/// Prepare one approved template release for chunk-by-chunk publication.
#[canic_update(internal, requires(caller::is_root()))]
async fn canic_wasm_store_prepare(
    request: TemplateChunkSetPrepareInput,
) -> Result<TemplateChunkSetInfoResponse, Error> {
    WasmStoreApi::prepare_chunk_set(request)
}

/// canic_wasm_store_publish_chunk
/// Publish one deterministic chunk into an already prepared local template release.
#[canic_update(internal, requires(caller::is_root()))]
async fn canic_wasm_store_publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
    WasmStoreApi::publish_chunk(request)
}

/// canic_wasm_store_info
/// Return deterministic chunk-set metadata for one local template release.
#[canic_update(internal, requires(caller::is_root()))]
async fn canic_wasm_store_info(
    template_id: TemplateId,
    version: TemplateVersion,
) -> Result<TemplateChunkSetInfoResponse, Error> {
    WasmStoreApi::template_info(template_id, version)
}

/// canic_wasm_store_status
/// Return occupied-byte and retention state for this local wasm store.
#[canic_query(internal, requires(caller::is_root()))]
async fn canic_wasm_store_status() -> Result<WasmStoreStatusResponse, Error> {
    WasmStoreApi::template_status(gc_state::snapshot())
}

/// canic_wasm_store_prepare_gc
/// Mark this local wasm store as prepared for store-local GC execution.
#[canic_update(internal, requires(caller::is_root()))]
async fn canic_wasm_store_prepare_gc() -> Result<(), Error> {
    gc_state::prepare(canic::cdk::api::time() / 1_000_000_000)
}

/// canic_wasm_store_begin_gc
/// Mark this local wasm store as actively executing store-local GC.
#[canic_update(internal, requires(caller::is_root()))]
async fn canic_wasm_store_begin_gc() -> Result<(), Error> {
    gc_state::begin(canic::cdk::api::time() / 1_000_000_000)
}

/// canic_wasm_store_complete_gc
/// Mark this local wasm store as having completed the current local GC pass.
#[canic_update(internal, requires(caller::is_root()))]
async fn canic_wasm_store_complete_gc() -> Result<(), Error> {
    let now_secs = canic::cdk::api::time() / 1_000_000_000;
    let current = gc_state::status();

    if current.mode == WasmStoreGcMode::Complete {
        return Ok(());
    }

    if current.mode != WasmStoreGcMode::InProgress {
        return Err(Error::conflict(format!(
            "wasm store gc transition {:?} -> Complete is not allowed",
            current.mode
        )));
    }

    let stats = WasmStoreApi::execute_local_store_gc().await?;
    gc_state::complete(now_secs)?;

    canic::log!(
        canic::api::ops::log::Topic::Wasm,
        Ok,
        "wasm_store: gc complete reclaimed_bytes={} cleared_templates={} cleared_releases={} cleared_chunks={} cleared_chunk_hashes={}",
        stats.reclaimed_store_bytes,
        stats.cleared_template_count,
        stats.cleared_release_count,
        stats.cleared_chunk_count,
        stats.cleared_chunk_store_hash_count
    );

    Ok(())
}

/// canic_wasm_store_chunk
/// Return one deterministic chunk for one local template release.
#[canic_update(internal, requires(caller::is_root()))]
async fn canic_wasm_store_chunk(
    template_id: TemplateId,
    version: TemplateVersion,
    chunk_index: u32,
) -> Result<TemplateChunkResponse, Error> {
    WasmStoreApi::template_chunk(template_id, version, chunk_index)
}

canic::cdk::export_candid_debug!();
