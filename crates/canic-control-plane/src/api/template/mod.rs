use crate::{
    dto::template::{
        TemplateChunkInput, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
        TemplateManifestInput,
    },
    ids::TemplateId,
    support,
};
#[cfg(feature = "wasm-store-canister")]
use crate::{
    dto::template::{
        TemplateChunkResponse, WasmStoreCatalogEntryResponse, WasmStoreStatusResponse,
    },
    ids::{TemplateVersion, WasmStoreGcMode, WasmStoreGcStatus},
    ops::storage::template::{WasmStoreGcExecutionStats, WasmStoreGcOps},
};
#[cfg(feature = "root-control-plane")]
use crate::{
    dto::template::{
        WasmStoreAdminCommand, WasmStoreAdminResponse, WasmStoreBootstrapDebugResponse,
        WasmStoreOverviewResponse,
    },
    ids::CanisterRole,
};
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
        support::stage_manifest(input);
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
        let response = support::prepare_chunk_set(request)?;
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
        support::publish_chunk(request)?;
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
        support::bootstrap_debug(&CanisterRole::WASM_STORE)
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
        support::publication_admin(cmd).await
    }

    // Return one root-owned overview for every tracked runtime-managed wasm store.
    pub fn overview() -> Result<WasmStoreOverviewResponse, Error> {
        Ok(support::publication_overview())
    }
}

///
/// LocalWasmStoreApi
///

#[cfg(feature = "wasm-store-canister")]
struct LocalWasmStoreApi;

#[cfg(feature = "wasm-store-canister")]
impl LocalWasmStoreApi {
    // Return the current approved release catalog stored in this local wasm store.
    fn template_catalog() -> Vec<WasmStoreCatalogEntryResponse> {
        support::local_template_catalog()
    }

    // Return occupied-byte and retention state for this local wasm store.
    fn template_status(gc: WasmStoreGcStatus) -> Result<WasmStoreStatusResponse, Error> {
        support::local_template_status(gc)
    }

    // Prepare one approved template release for chunk-by-chunk publication in this local wasm store.
    fn prepare_chunk_set(
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        support::local_prepare_chunk_set(request)
    }

    // Stage one approved manifest in this local wasm store.
    fn stage_manifest(request: TemplateManifestInput) -> Result<(), Error> {
        support::local_stage_manifest(request)
    }

    // Publish one deterministic chunk into an already prepared local template release.
    fn publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
        support::local_publish_chunk(request)
    }

    // Clear all local template records, chunk metadata, and staged chunk hashes for store-local GC.
    async fn execute_local_store_gc() -> Result<WasmStoreGcExecutionStats, Error> {
        support::execute_local_store_gc().await
    }

    // Return deterministic chunk-set metadata for one local template release.
    fn template_info(
        template_id: TemplateId,
        version: TemplateVersion,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        support::local_template_info(template_id, version)
    }

    // Return one deterministic chunk for one local template release.
    fn template_chunk(
        template_id: TemplateId,
        version: TemplateVersion,
        chunk_index: u32,
    ) -> Result<TemplateChunkResponse, Error> {
        support::local_template_chunk(template_id, version, chunk_index)
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
        Ok(LocalWasmStoreApi::template_catalog())
    }

    // Prepare one approved template release for chunk-by-chunk publication.
    pub fn prepare(
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        LocalWasmStoreApi::prepare_chunk_set(request)
    }

    // Stage one approved manifest in this local wasm store.
    pub fn stage_manifest(request: TemplateManifestInput) -> Result<(), Error> {
        LocalWasmStoreApi::stage_manifest(request)
    }

    // Publish one deterministic chunk into an already prepared local template release.
    pub fn publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
        LocalWasmStoreApi::publish_chunk(request)
    }

    // Return deterministic chunk-set metadata for one local template release.
    pub fn info(
        template_id: TemplateId,
        version: TemplateVersion,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        LocalWasmStoreApi::template_info(template_id, version)
    }

    // Return occupied-byte and retention state for this local wasm store.
    pub fn status() -> Result<WasmStoreStatusResponse, Error> {
        LocalWasmStoreApi::template_status(WasmStoreGcOps::snapshot())
    }

    // Mark this local wasm store as prepared for store-local GC execution.
    pub fn prepare_gc() -> Result<(), Error> {
        WasmStoreGcOps::prepare(support::now_secs())
    }

    // Mark this local wasm store as actively executing store-local GC.
    pub fn begin_gc() -> Result<(), Error> {
        WasmStoreGcOps::begin(support::now_secs())
    }

    // Mark this local wasm store as having completed the current local GC pass.
    pub async fn complete_gc() -> Result<(), Error> {
        let now_secs = support::now_secs();
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

        WasmStoreGcOps::begin_clearing(now_secs)?;
        let stats = match LocalWasmStoreApi::execute_local_store_gc().await {
            Ok(stats) => stats,
            Err(err) => {
                let _ = WasmStoreGcOps::begin(support::now_secs());
                return Err(err);
            }
        };
        WasmStoreGcOps::complete(support::now_secs())?;

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
        LocalWasmStoreApi::template_chunk(template_id, version, chunk_index)
    }
}
