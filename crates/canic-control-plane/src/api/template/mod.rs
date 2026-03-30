use crate::{
    dto::template::{
        TemplateChunkInput, TemplateChunkResponse, TemplateChunkSetInfoResponse,
        TemplateChunkSetPrepareInput, TemplateManifestInput, WasmStoreAdminCommand,
        WasmStoreAdminResponse, WasmStoreBootstrapDebugResponse, WasmStoreCatalogEntryResponse,
        WasmStoreOverviewResponse, WasmStoreStatusResponse,
    },
    ids::{CanisterRole, TemplateId, TemplateVersion, WasmStoreBinding, WasmStoreGcStatus},
    support::{self, WasmStoreGcExecutionStats},
};
use canic_core::{cdk::types::Principal, dto::error::Error};

const ROOT_WASM_STORE_BOOTSTRAP_TEMPLATE_ID: TemplateId = TemplateId::new("embedded:wasm_store");
const ROOT_WASM_STORE_BOOTSTRAP_BINDING: WasmStoreBinding = WasmStoreBinding::new("bootstrap");

///
/// WasmStoreBootstrapApi
///

pub struct WasmStoreBootstrapApi;

impl WasmStoreBootstrapApi {
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
        support::stage_manifest(input);
    }

    // Prepare one local chunk set for chunk-by-chunk staging in the current canister.
    pub fn prepare_chunk_set(
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        support::prepare_chunk_set(request)
    }

    // Stage one chunk into the current canister's local bootstrap source.
    pub fn publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
        support::publish_chunk(request)
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
