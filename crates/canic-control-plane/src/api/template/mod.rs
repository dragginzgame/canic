#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
use crate::{
    config,
    ids::WasmStoreGcStatus,
    ops::storage::template::{TemplateChunkedOps, TemplateManifestOps, WasmStoreLimits},
};
use crate::{
    dto::template::{
        TemplateChunkInput, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
        TemplateManifestInput,
    },
    ids::TemplateId,
};
#[cfg(feature = "wasm-store-canister")]
use crate::{
    dto::template::{
        TemplateChunkResponse, WasmStoreCatalogEntryResponse, WasmStoreStatusResponse,
    },
    ids::{TemplateVersion, WasmStoreGcMode},
    ops::storage::template::{WasmStoreGcExecutionStats, WasmStoreGcOps},
};
#[cfg(feature = "root-control-plane")]
use crate::{
    dto::template::{
        WasmStoreAdminCommand, WasmStoreAdminResponse, WasmStoreBootstrapDebugResponse,
        WasmStoreOverviewResponse, WasmStorePublicationSlotResponse,
    },
    ids::CanisterRole,
    ops::storage::state::subnet::SubnetStateOps,
    workflow::runtime::template::WasmStorePublicationWorkflow,
};
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
use canic_core::control_plane_support::ops::ic::IcOps;
#[cfg(feature = "root-control-plane")]
use canic_core::{
    api::runtime::install::ModuleSourceRuntimeApi, bootstrap::EmbeddedRootBootstrapEntry,
};
use canic_core::{dto::error::Error, log, log::Topic};

#[cfg(feature = "root-control-plane")]
const ROOT_WASM_STORE_BOOTSTRAP_TEMPLATE_ID: TemplateId = TemplateId::new("embedded:wasm_store");

///
/// WasmStoreBootstrapApi
///

#[cfg(feature = "root-control-plane")]
pub struct WasmStoreBootstrapApi;

#[cfg(feature = "root-control-plane")]
impl WasmStoreBootstrapApi {
    // Register the dedicated embedded bootstrap release set used for the first live store install.
    pub fn register_embedded_root_wasm_store_release_set(
        entries: &'static [EmbeddedRootBootstrapEntry],
    ) {
        let Some(entry) = entries
            .iter()
            .find(|entry| entry.role == CanisterRole::WASM_STORE.as_str())
        else {
            return;
        };

        ModuleSourceRuntimeApi::register_embedded_module_wasm(
            CanisterRole::WASM_STORE,
            ROOT_WASM_STORE_BOOTSTRAP_TEMPLATE_ID.as_str().to_string(),
            entry.wasm_module,
        );
    }

    // Log the exact embedded bootstrap artifact provenance captured during root build.
    pub fn log_embedded_root_wasm_store_release_set(
        entries: &'static [EmbeddedRootBootstrapEntry],
    ) {
        let Some(entry) = entries
            .iter()
            .find(|entry| entry.role == CanisterRole::WASM_STORE.as_str())
        else {
            return;
        };

        log!(
            Topic::Init,
            Info,
            "ws bootstrap artifact: source_path={} embedded_path={} kind={} bytes={} sha256={} decompressed_bytes={:?} decompressed_sha256={:?}",
            entry.artifact_path,
            entry.embedded_artifact_path,
            entry.artifact_kind,
            entry.artifact_size_bytes,
            entry.artifact_sha256_hex,
            entry.decompressed_size_bytes,
            entry.decompressed_sha256_hex,
        );
    }

    // Stage one approved manifest in the current canister's local bootstrap source.
    pub fn stage_manifest(input: TemplateManifestInput) {
        log!(
            Topic::Wasm,
            Info,
            "stage manifest template={} role={} version={} bytes={} binding={}",
            input.template_id,
            input.role,
            input.version,
            input.payload_size_bytes,
            input.store_binding,
        );
        stage_bootstrap_manifest(input);
    }

    // Prepare one local chunk set for chunk-by-chunk staging in the current canister.
    pub fn prepare_chunk_set(
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        let template_id = request.template_id.clone();
        let version = request.version.clone();
        log!(
            Topic::Wasm,
            Info,
            "prepare chunk upload template={} version={} chunks={} bytes={}",
            request.template_id,
            request.version,
            request.chunk_hashes.len(),
            request.payload_size_bytes,
        );
        let response = prepare_bootstrap_chunk_set(request)?;
        log!(
            Topic::Wasm,
            Ok,
            "prepared chunk upload template={} version={} with {} chunk hashes",
            template_id,
            version,
            response.chunk_hashes.len(),
        );
        Ok(response)
    }

    // Stage one chunk into the current canister's local bootstrap source.
    pub fn publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
        log!(
            Topic::Wasm,
            Info,
            "publish chunk template={} version={} chunk_index={} bytes={}",
            request.template_id,
            request.version,
            request.chunk_index,
            request.bytes.len(),
        );
        let template_id = request.template_id.clone();
        let version = request.version.clone();
        let chunk_index = request.chunk_index;
        publish_bootstrap_chunk(request)?;
        log!(
            Topic::Wasm,
            Ok,
            "published chunk template={} version={} chunk_index={}",
            template_id,
            version,
            chunk_index,
        );
        Ok(())
    }

    // Return root-owned staged bootstrap visibility for the bootstrap role and current release buffer.
    pub fn debug_bootstrap() -> Result<WasmStoreBootstrapDebugResponse, Error> {
        bootstrap_debug(&CanisterRole::WASM_STORE)
    }
}

///
/// WasmStorePublicationApi
///

#[cfg(feature = "root-control-plane")]
pub struct WasmStorePublicationApi;

#[cfg(feature = "root-control-plane")]
impl WasmStorePublicationApi {
    // Execute one typed root-owned WasmStore publication or lifecycle admin command.
    pub async fn admin(cmd: WasmStoreAdminCommand) -> Result<WasmStoreAdminResponse, Error> {
        publication_admin(cmd).await
    }

    // Return one root-owned overview for every tracked runtime-managed wasm store.
    pub fn overview() -> Result<WasmStoreOverviewResponse, Error> {
        Ok(publication_overview())
    }
}

///
/// WasmStoreCanisterApi
///

#[cfg(feature = "wasm-store-canister")]
pub struct WasmStoreCanisterApi;

#[cfg(feature = "wasm-store-canister")]
impl WasmStoreCanisterApi {
    // Return the current approved release catalog stored in this local wasm store.
    pub fn catalog() -> Result<Vec<WasmStoreCatalogEntryResponse>, Error> {
        Ok(local_template_catalog())
    }

    // Prepare one approved template release for chunk-by-chunk publication.
    pub fn prepare(
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        local_prepare_chunk_set(request)
    }

    // Stage one approved manifest in this local wasm store.
    pub fn stage_manifest(request: TemplateManifestInput) -> Result<(), Error> {
        local_stage_manifest(request)
    }

    // Publish one deterministic chunk into an already prepared local template release.
    pub fn publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
        local_publish_chunk(request)
    }

    // Return deterministic chunk-set metadata for one local template release.
    pub fn info(
        template_id: TemplateId,
        version: TemplateVersion,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        local_template_info(template_id, version)
    }

    // Return occupied-byte and retention state for this local wasm store.
    pub fn status() -> Result<WasmStoreStatusResponse, Error> {
        local_template_status(WasmStoreGcOps::snapshot())
    }

    // Mark this local wasm store as prepared for store-local GC execution.
    pub fn prepare_gc() -> Result<(), Error> {
        WasmStoreGcOps::prepare(now_secs())
    }

    // Mark this local wasm store as actively executing store-local GC.
    pub fn begin_gc() -> Result<(), Error> {
        WasmStoreGcOps::begin(now_secs())
    }

    // Mark this local wasm store as having completed the current local GC pass.
    pub async fn complete_gc() -> Result<(), Error> {
        let clearing_started_at = now_secs();
        let current = WasmStoreGcOps::status();

        if current.mode == WasmStoreGcMode::Complete {
            return Ok(());
        }

        if current.mode != WasmStoreGcMode::InProgress {
            return Err(Error::conflict(format!(
                "wasm store gc transition {:?} -> Complete is not allowed",
                current.mode
            )));
        }

        WasmStoreGcOps::begin_clearing(clearing_started_at)?;
        let stats = match execute_local_store_gc().await {
            Ok(stats) => stats,
            Err(err) => {
                let _ = WasmStoreGcOps::begin(now_secs());
                return Err(err);
            }
        };
        WasmStoreGcOps::complete(now_secs())?;

        log!(
            Topic::Wasm,
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

    // Return one deterministic chunk for one local template release.
    pub fn chunk(
        template_id: TemplateId,
        version: TemplateVersion,
        chunk_index: u32,
    ) -> Result<TemplateChunkResponse, Error> {
        local_template_chunk(template_id, version, chunk_index)
    }
}

#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
fn now_secs() -> u64 {
    IcOps::now_secs()
}

#[cfg(feature = "root-control-plane")]
fn stage_bootstrap_manifest(input: TemplateManifestInput) {
    TemplateManifestOps::replace_approved_from_input(input);
}

#[cfg(feature = "root-control-plane")]
fn prepare_bootstrap_chunk_set(
    request: TemplateChunkSetPrepareInput,
) -> Result<TemplateChunkSetInfoResponse, Error> {
    TemplateChunkedOps::prepare_chunk_set_from_input(request, now_secs()).map_err(Error::from)
}

#[cfg(feature = "root-control-plane")]
fn publish_bootstrap_chunk(request: TemplateChunkInput) -> Result<(), Error> {
    TemplateChunkedOps::publish_chunk_from_input(request).map_err(Error::from)
}

#[cfg(feature = "root-control-plane")]
fn bootstrap_debug(
    bootstrap_role: &CanisterRole,
) -> Result<WasmStoreBootstrapDebugResponse, Error> {
    TemplateChunkedOps::bootstrap_debug_response(bootstrap_role).map_err(Error::from)
}

#[cfg(feature = "root-control-plane")]
async fn publication_admin(cmd: WasmStoreAdminCommand) -> Result<WasmStoreAdminResponse, Error> {
    WasmStorePublicationWorkflow::handle_admin(cmd)
        .await
        .map_err(Error::from)
}

#[cfg(feature = "root-control-plane")]
fn publication_overview() -> WasmStoreOverviewResponse {
    let store = config::current_subnet_default_wasm_store();
    let limits = WasmStoreLimits {
        max_store_bytes: store.max_store_bytes(),
        max_templates: store.max_templates(),
        max_template_versions_per_template: store.max_template_versions_per_template(),
    };
    let headroom_bytes = store.headroom_bytes();
    let publication = SubnetStateOps::publication_store_state_response();
    let stores = SubnetStateOps::wasm_stores()
        .into_iter()
        .map(|store| {
            let publication_slot = if publication.active_binding.as_ref() == Some(&store.binding) {
                Some(WasmStorePublicationSlotResponse::Active)
            } else if publication.detached_binding.as_ref() == Some(&store.binding) {
                Some(WasmStorePublicationSlotResponse::Detached)
            } else if publication.retired_binding.as_ref() == Some(&store.binding) {
                Some(WasmStorePublicationSlotResponse::Retired)
            } else {
                None
            };

            TemplateManifestOps::root_store_overview_response(
                &store.binding,
                store.pid,
                store.created_at,
                limits,
                headroom_bytes,
                WasmStoreGcStatus {
                    mode: store.gc.mode,
                    changed_at: store.gc.changed_at,
                    prepared_at: store.gc.prepared_at,
                    started_at: store.gc.started_at,
                    completed_at: store.gc.completed_at,
                    runs_completed: store.gc.runs_completed,
                },
                publication_slot,
            )
        })
        .collect();

    WasmStoreOverviewResponse {
        publication,
        stores,
    }
}

#[cfg(feature = "wasm-store-canister")]
fn local_template_catalog() -> Vec<WasmStoreCatalogEntryResponse> {
    TemplateManifestOps::approved_catalog_response()
}

#[cfg(feature = "wasm-store-canister")]
fn local_template_status(gc: WasmStoreGcStatus) -> Result<WasmStoreStatusResponse, Error> {
    let store = config::current_wasm_store().map_err(Error::from)?;
    let limits = WasmStoreLimits {
        max_store_bytes: store.max_store_bytes(),
        max_templates: store.max_templates(),
        max_template_versions_per_template: store.max_template_versions_per_template(),
    };
    Ok(TemplateChunkedOps::store_status_response(
        limits,
        store.headroom_bytes(),
        gc,
    ))
}

#[cfg(feature = "wasm-store-canister")]
fn local_prepare_chunk_set(
    request: TemplateChunkSetPrepareInput,
) -> Result<TemplateChunkSetInfoResponse, Error> {
    let store = config::current_wasm_store().map_err(Error::from)?;
    let limits = WasmStoreLimits {
        max_store_bytes: store.max_store_bytes(),
        max_templates: store.max_templates(),
        max_template_versions_per_template: store.max_template_versions_per_template(),
    };
    TemplateChunkedOps::prepare_chunk_set_in_store_from_input(request, now_secs(), limits)
        .map_err(Error::from)
}

#[cfg(feature = "wasm-store-canister")]
fn local_stage_manifest(request: TemplateManifestInput) -> Result<(), Error> {
    let store = config::current_wasm_store().map_err(Error::from)?;
    let limits = WasmStoreLimits {
        max_store_bytes: store.max_store_bytes(),
        max_templates: store.max_templates(),
        max_template_versions_per_template: store.max_template_versions_per_template(),
    };
    TemplateChunkedOps::replace_approved_in_store_from_input(request, limits).map_err(Error::from)
}

#[cfg(feature = "wasm-store-canister")]
fn local_publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
    let store = config::current_wasm_store().map_err(Error::from)?;
    let limits = WasmStoreLimits {
        max_store_bytes: store.max_store_bytes(),
        max_templates: store.max_templates(),
        max_template_versions_per_template: store.max_template_versions_per_template(),
    };
    TemplateChunkedOps::publish_chunk_in_store_from_input(request, limits).map_err(Error::from)
}

#[cfg(feature = "wasm-store-canister")]
async fn execute_local_store_gc() -> Result<WasmStoreGcExecutionStats, Error> {
    TemplateChunkedOps::execute_local_store_gc()
        .await
        .map_err(Error::from)
}

#[cfg(feature = "wasm-store-canister")]
fn local_template_info(
    template_id: TemplateId,
    version: TemplateVersion,
) -> Result<TemplateChunkSetInfoResponse, Error> {
    TemplateChunkedOps::chunk_set_info_response(&template_id, &version).map_err(Error::from)
}

#[cfg(feature = "wasm-store-canister")]
fn local_template_chunk(
    template_id: TemplateId,
    version: TemplateVersion,
    chunk_index: u32,
) -> Result<TemplateChunkResponse, Error> {
    TemplateChunkedOps::chunk_response(&template_id, &version, chunk_index).map_err(Error::from)
}
