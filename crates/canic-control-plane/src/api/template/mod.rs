use crate::{
    dto::template::{
        TemplateChunkInput, TemplateChunkResponse, TemplateChunkSetInfoResponse,
        TemplateChunkSetPrepareInput, TemplateManifestInput, WasmStoreAdminCommand,
        WasmStoreAdminResponse, WasmStoreBootstrapDebugResponse, WasmStoreCatalogEntryResponse,
        WasmStoreOverviewResponse, WasmStorePublicationStatusResponse,
        WasmStoreRetiredStoreStatusResponse, WasmStoreStatusResponse,
    },
    ids::{
        CanisterRole, TemplateId, TemplateVersion, WasmStoreBinding, WasmStoreGcMode,
        WasmStoreGcStatus,
    },
    ops::storage::template::WasmStoreGcOps,
    support::{self, WasmStoreGcExecutionStats},
};
use canic_core::{
    api::runtime::install::ModuleSourceRuntimeApi, bootstrap::EmbeddedRootBootstrapEntry,
    cdk::types::Principal, dto::error::Error, log, log::Topic,
};

const ROOT_WASM_STORE_BOOTSTRAP_TEMPLATE_ID: TemplateId = TemplateId::new("embedded:wasm_store");
const ROOT_WASM_STORE_BOOTSTRAP_BINDING: WasmStoreBinding = WasmStoreBinding::new("bootstrap");

///
/// WasmStoreBootstrapApi
///

pub struct WasmStoreBootstrapApi;

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

    // Validate that one staged template request targets the root-local WasmStore bootstrap source.
    fn ensure_root_wasm_store_bootstrap_template(template_id: &TemplateId) -> Result<(), Error> {
        if template_id == &ROOT_WASM_STORE_BOOTSTRAP_TEMPLATE_ID {
            Ok(())
        } else {
            Err(Error::invalid(format!(
                "bootstrap only accepts template '{ROOT_WASM_STORE_BOOTSTRAP_TEMPLATE_ID}'"
            )))
        }
    }

    // Normalize one staged manifest onto the root-local WasmStore bootstrap source of truth.
    fn normalize_root_wasm_store_bootstrap_manifest(
        request: TemplateManifestInput,
    ) -> Result<TemplateManifestInput, Error> {
        if request.role != CanisterRole::WASM_STORE {
            return Err(Error::invalid(format!(
                "bootstrap only accepts role '{}'",
                CanisterRole::WASM_STORE
            )));
        }

        Self::ensure_root_wasm_store_bootstrap_template(&request.template_id)?;

        let now_secs = support::now_secs();

        Ok(TemplateManifestInput {
            template_id: ROOT_WASM_STORE_BOOTSTRAP_TEMPLATE_ID,
            role: CanisterRole::WASM_STORE,
            version: request.version,
            payload_hash: request.payload_hash,
            payload_size_bytes: request.payload_size_bytes,
            store_binding: ROOT_WASM_STORE_BOOTSTRAP_BINDING,
            chunking_mode: crate::ids::TemplateChunkingMode::Chunked,
            manifest_state: crate::ids::TemplateManifestState::Approved,
            approved_at: Some(now_secs),
            created_at: now_secs,
        })
    }

    // Stage the normalized root-local bootstrap manifest for `embedded:wasm_store`.
    pub fn stage_root_wasm_store_manifest(request: TemplateManifestInput) -> Result<(), Error> {
        Self::stage_manifest(Self::normalize_root_wasm_store_bootstrap_manifest(request)?);
        Ok(())
    }

    // Prepare root-local chunk metadata for the staged `embedded:wasm_store` bootstrap source.
    pub fn prepare_root_wasm_store_chunk_set(
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        Self::ensure_root_wasm_store_bootstrap_template(&request.template_id)?;
        Self::prepare_chunk_set(request)
    }

    // Publish one root-local chunk into the staged `embedded:wasm_store` bootstrap source.
    pub fn publish_root_wasm_store_chunk(request: TemplateChunkInput) -> Result<(), Error> {
        Self::ensure_root_wasm_store_bootstrap_template(&request.template_id)?;
        Self::publish_chunk(request)
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

    // Publish all root-local staged releases into the current subnet's selected wasm store.
    pub async fn publish_staged_release_set_to_current_store() -> Result<(), Error> {
        support::publish_staged_release_set_to_current_store().await
    }

    // Return root-owned staged bootstrap visibility for the bootstrap role and current release buffer.
    pub fn debug_bootstrap() -> Result<WasmStoreBootstrapDebugResponse, Error> {
        support::bootstrap_debug(&CanisterRole::WASM_STORE)
    }
}

///
/// WasmStorePublicationApi
///

pub struct WasmStorePublicationApi;

impl WasmStorePublicationApi {
    // Execute one typed root-owned WasmStore publication or lifecycle admin command.
    pub async fn admin(cmd: WasmStoreAdminCommand) -> Result<WasmStoreAdminResponse, Error> {
        support::publication_admin(cmd).await
    }

    // Publish the current release set into one subnet-local wasm store.
    pub async fn publish_current_release_set_to_store(store_pid: Principal) -> Result<(), Error> {
        support::publish_current_release_set_to_store(store_pid).await
    }

    // Publish the current release set into the current subnet's selected publication wasm store.
    pub async fn publish_current_release_set_to_current_store() -> Result<(), Error> {
        support::publish_current_release_set_to_current_store().await
    }

    // Persist one explicit publication binding for the current subnet.
    pub fn set_current_publication_store_binding(binding: WasmStoreBinding) -> Result<(), Error> {
        support::set_current_publication_store_binding(binding)
    }

    // Clear the explicit publication binding for the current subnet.
    pub fn clear_current_publication_store_binding() {
        support::clear_current_publication_store_binding();
    }

    // Retire the current detached publication binding for the current subnet.
    #[must_use]
    pub fn retire_detached_publication_store_binding() -> Option<WasmStoreBinding> {
        support::retire_detached_publication_store_binding()
    }

    // Return one root-owned overview for every tracked runtime-managed wasm store.
    pub fn overview() -> Result<WasmStoreOverviewResponse, Error> {
        Ok(support::publication_overview())
    }

    // Return one live root-facing publication placement snapshot for the managed store fleet.
    pub async fn status() -> Result<WasmStorePublicationStatusResponse, Error> {
        support::publication_status().await
    }

    // Return the current retired runtime-managed publication store status, if one exists.
    pub async fn retired_store_status() -> Result<Option<WasmStoreRetiredStoreStatusResponse>, Error>
    {
        support::retired_publication_store_status().await
    }

    // Mark the current retired publication store as prepared for store-local GC execution.
    pub async fn prepare_retired_publication_store_for_gc()
    -> Result<Option<WasmStoreBinding>, Error> {
        support::prepare_retired_publication_store_for_gc().await
    }

    // Mark the current retired publication store as actively executing store-local GC.
    pub async fn begin_retired_publication_store_gc() -> Result<Option<WasmStoreBinding>, Error> {
        support::begin_retired_publication_store_gc().await
    }

    // Mark the current retired publication store as having completed its local GC pass.
    pub async fn complete_retired_publication_store_gc() -> Result<Option<WasmStoreBinding>, Error>
    {
        support::complete_retired_publication_store_gc().await
    }

    // Finalize the retired publication binding once store-local GC has completed.
    pub async fn finalize_retired_publication_store_binding()
    -> Result<Option<WasmStoreBinding>, Error> {
        support::finalize_retired_publication_store_binding().await
    }

    // Delete one finalized runtime-managed publication store canister.
    pub async fn delete_finalized_publication_store(
        binding: WasmStoreBinding,
        store_pid: Principal,
    ) -> Result<(), Error> {
        support::delete_finalized_publication_store(binding, store_pid).await
    }
}

///
/// WasmStoreApi
///

pub struct WasmStoreApi;

impl WasmStoreApi {
    // Return the current approved release catalog stored in this local wasm store.
    pub fn template_catalog() -> Result<Vec<WasmStoreCatalogEntryResponse>, Error> {
        Ok(support::local_template_catalog())
    }

    // Return occupied-byte and retention state for this local wasm store.
    pub fn template_status(gc: WasmStoreGcStatus) -> Result<WasmStoreStatusResponse, Error> {
        support::local_template_status(gc)
    }

    // Prepare one approved template release for chunk-by-chunk publication in this local wasm store.
    pub fn prepare_chunk_set(
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        support::local_prepare_chunk_set(request)
    }

    // Stage one approved manifest in this local wasm store.
    pub fn stage_manifest(request: TemplateManifestInput) -> Result<(), Error> {
        support::local_stage_manifest(request)
    }

    // Publish one deterministic chunk into an already prepared local template release.
    pub fn publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
        support::local_publish_chunk(request)
    }

    // Clear all local template records, chunk metadata, and staged chunk hashes for store-local GC.
    pub async fn execute_local_store_gc() -> Result<WasmStoreGcExecutionStats, Error> {
        support::execute_local_store_gc().await
    }

    // Return deterministic chunk-set metadata for one local template release.
    pub fn template_info(
        template_id: TemplateId,
        version: TemplateVersion,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        support::local_template_info(template_id, version)
    }

    // Return one deterministic chunk for one local template release.
    pub fn template_chunk(
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

pub struct WasmStoreCanisterApi;

impl WasmStoreCanisterApi {
    // Return the current approved release catalog stored in this local wasm store.
    pub fn catalog() -> Result<Vec<WasmStoreCatalogEntryResponse>, Error> {
        WasmStoreApi::template_catalog()
    }

    // Prepare one approved template release for chunk-by-chunk publication.
    pub fn prepare(
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        WasmStoreApi::prepare_chunk_set(request)
    }

    // Stage one approved manifest in this local wasm store.
    pub fn stage_manifest(request: TemplateManifestInput) -> Result<(), Error> {
        WasmStoreApi::stage_manifest(request)
    }

    // Publish one deterministic chunk into an already prepared local template release.
    pub fn publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
        WasmStoreApi::publish_chunk(request)
    }

    // Return deterministic chunk-set metadata for one local template release.
    pub fn info(
        template_id: TemplateId,
        version: TemplateVersion,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        WasmStoreApi::template_info(template_id, version)
    }

    // Return occupied-byte and retention state for this local wasm store.
    pub fn status() -> Result<WasmStoreStatusResponse, Error> {
        WasmStoreApi::template_status(WasmStoreGcOps::snapshot())
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

        let stats = WasmStoreApi::execute_local_store_gc().await?;
        WasmStoreGcOps::complete(now_secs)?;

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
        WasmStoreApi::template_chunk(template_id, version, chunk_index)
    }
}
