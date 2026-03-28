//!
//! Subnet-local wasm store canister for approved template chunk sets.
//! Lives in `crates/canisters` as the storage-only `0.18` wasm store role.
//!

#![allow(clippy::unused_async)]

mod gc_state;

use canic::{
    Error,
    api::canister::template::WasmStoreApi,
    dto::template::{
        TemplateChunkInput, TemplateChunkResponse, TemplateChunkSetInfoResponse,
        TemplateChunkSetPrepareInput, WasmStoreCatalogEntryResponse, WasmStoreStatusResponse,
    },
    ids::{TemplateId, TemplateVersion},
    prelude::*,
};
use canic_internal::canister;

// Seed the local store from embedded releases before bootstrap timers fire.
//
// CANIC
//

canic::start!(
    canister::WASM_STORE,
    init = {
        WasmStoreApi::import_embedded_release_set(EMBEDDED_RELEASE_SET);
    }
);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

//
// ENDPOINTS
//

//
// EMBEDDED RELEASE TABLE
//

#[cfg(target_arch = "wasm32")]
const APP_WASM: &[u8] = include_bytes!("../../../../.dfx/local/canisters/app/app.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const APP_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const USER_HUB_WASM: &[u8] =
    include_bytes!("../../../../.dfx/local/canisters/user_hub/user_hub.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const USER_HUB_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const USER_SHARD_WASM: &[u8] =
    include_bytes!("../../../../.dfx/local/canisters/user_shard/user_shard.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const USER_SHARD_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const MINIMAL_WASM: &[u8] =
    include_bytes!("../../../../.dfx/local/canisters/minimal/minimal.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const MINIMAL_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const SCALE_HUB_WASM: &[u8] =
    include_bytes!("../../../../.dfx/local/canisters/scale_hub/scale_hub.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const SCALE_HUB_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const SCALE_WASM: &[u8] = include_bytes!("../../../../.dfx/local/canisters/scale/scale.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const SCALE_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const SHARD_HUB_WASM: &[u8] =
    include_bytes!("../../../../.dfx/local/canisters/shard_hub/shard_hub.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const SHARD_HUB_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const SHARD_WASM: &[u8] = include_bytes!("../../../../.dfx/local/canisters/shard/shard.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const SHARD_WASM: &[u8] = &[];

#[cfg(target_arch = "wasm32")]
const TEST_WASM: &[u8] = include_bytes!("../../../../.dfx/local/canisters/test/test.wasm.gz");
#[cfg(not(target_arch = "wasm32"))]
const TEST_WASM: &[u8] = &[];

const EMBEDDED_RELEASE_SET: &[(CanisterRole, &[u8])] = &[
    (canister::APP, APP_WASM),
    (canister::USER_HUB, USER_HUB_WASM),
    (canister::USER_SHARD, USER_SHARD_WASM),
    (canister::MINIMAL, MINIMAL_WASM),
    (canister::SCALE_HUB, SCALE_HUB_WASM),
    (canister::SCALE, SCALE_WASM),
    (canister::SHARD_HUB, SHARD_HUB_WASM),
    (canister::SHARD, SHARD_WASM),
    (canister::TEST, TEST_WASM),
];

/// canic_wasm_store_catalog
/// Return the approved embedded release catalog for this local wasm store.
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

    if current.mode == canic::ids::WasmStoreGcMode::Complete {
        return Ok(());
    }

    if current.mode != canic::ids::WasmStoreGcMode::InProgress {
        return Err(Error::conflict(format!(
            "wasm store gc transition {:?} -> Complete is not allowed",
            current.mode
        )));
    }

    let stats = WasmStoreApi::execute_local_store_gc().await?;
    gc_state::complete(now_secs)?;

    canic::log!(
        canic::api::ops::log::Topic::Wasm,
        Warn,
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

#[cfg(debug_assertions)]
export_candid!();
