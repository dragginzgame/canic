use crate::{
    cdk::types::Principal,
    dto::{
        error::Error,
        template::{
            TemplateChunkInput, TemplateChunkResponse, TemplateChunkSetInfoResponse,
            TemplateChunkSetPrepareInput, TemplateManifestInput, WasmStoreAdminCommand,
            WasmStoreAdminResponse, WasmStoreCatalogEntryResponse, WasmStoreOverviewResponse,
            WasmStorePublicationSlotResponse, WasmStorePublicationStateResponse,
            WasmStoreRetiredStoreStatusResponse, WasmStoreStatusResponse,
        },
    },
    ids::{CanisterRole, TemplateId, TemplateVersion, WasmStoreBinding, WasmStoreGcStatus},
    ops::{
        config::ConfigOps,
        ic::IcOps,
        runtime::template::WasmStoreCatalogOps,
        storage::{
            state::subnet::SubnetStateOps,
            template::{TemplateManifestOps, WasmStoreGcExecutionStats, WasmStoreLimits},
        },
    },
    workflow::runtime::template::WasmStorePublicationWorkflow,
};

const ROOT_WASM_STORE_BOOTSTRAP_TEMPLATE_ID: TemplateId = TemplateId::new("embedded:wasm_store");
const ROOT_WASM_STORE_BOOTSTRAP_BINDING: WasmStoreBinding = WasmStoreBinding::new("bootstrap");

///
/// EmbeddedTemplateApi
///

pub struct EmbeddedTemplateApi;

impl EmbeddedTemplateApi {
    // Seed approved manifests and template-keyed embedded payloads for the current release set.
    pub fn import_embedded_release_set(wasms: &'static [(CanisterRole, &[u8])]) {
        WasmStorePublicationWorkflow::import_embedded_release_set(wasms);
    }
}

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

        let now_secs = IcOps::now_secs();

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

    // Seed the compact embedded release catalog used for root manifest bootstrap.
    pub fn import_embedded_release_catalog(entries: Vec<WasmStoreCatalogEntryResponse>) {
        WasmStoreCatalogOps::import_embedded(entries);
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
        TemplateManifestOps::replace_approved_from_input(input);
    }

    // Prepare one local chunk set for chunk-by-chunk staging in the current canister.
    pub fn prepare_chunk_set(
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        let now_secs = IcOps::now_secs();
        TemplateManifestOps::prepare_chunk_set_from_input(request, now_secs).map_err(Error::from)
    }

    // Stage one chunk into the current canister's local bootstrap source.
    pub fn publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
        TemplateManifestOps::publish_chunk_from_input(request).map_err(Error::from)
    }
}

///
/// WasmStorePublicationApi
///

pub struct WasmStorePublicationApi;

impl WasmStorePublicationApi {
    // Execute one typed root-owned WasmStore publication or lifecycle admin command.
    pub async fn admin(cmd: WasmStoreAdminCommand) -> Result<WasmStoreAdminResponse, Error> {
        WasmStorePublicationWorkflow::handle_admin(cmd)
            .await
            .map_err(Error::from)
    }

    // Publish the current release set into one subnet-local wasm store.
    pub async fn publish_current_release_set_to_store(store_pid: Principal) -> Result<(), Error> {
        WasmStorePublicationWorkflow::publish_current_release_set_to_store(store_pid)
            .await
            .map_err(Error::from)
    }

    // Publish the current release set into the current subnet's selected publication wasm store.
    pub async fn publish_current_release_set_to_current_store() -> Result<(), Error> {
        WasmStorePublicationWorkflow::publish_current_release_set_to_current_store()
            .await
            .map_err(Error::from)
    }

    // Persist one explicit publication binding for the current subnet.
    pub fn set_current_publication_store_binding(binding: WasmStoreBinding) -> Result<(), Error> {
        WasmStorePublicationWorkflow::set_current_publication_store_binding(binding)
            .map_err(Error::from)
    }

    // Clear the explicit publication binding for the current subnet.
    pub fn clear_current_publication_store_binding() {
        WasmStorePublicationWorkflow::clear_current_publication_store_binding();
    }

    // Retire the current detached publication binding for the current subnet.
    #[must_use]
    pub fn retire_detached_publication_store_binding() -> Option<WasmStoreBinding> {
        WasmStorePublicationWorkflow::retire_detached_publication_store_binding()
    }

    // Return the current publication-store lifecycle state for the current subnet.
    #[must_use]
    pub fn publication_store_state() -> WasmStorePublicationStateResponse {
        SubnetStateOps::publication_store_state_response()
    }

    // Return one root-owned overview for every tracked runtime-managed wasm store.
    pub fn overview() -> Result<WasmStoreOverviewResponse, Error> {
        let publication = SubnetStateOps::publication_store_state_response();
        let limits = WasmStoreApi::current_store_limits()?;
        let headroom_bytes = WasmStoreApi::current_store_headroom_bytes()?;
        let stores = SubnetStateOps::wasm_stores()
            .into_iter()
            .map(|store| {
                let publication_slot =
                    if publication.active_binding.as_ref() == Some(&store.binding) {
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
                    crate::ids::WasmStoreGcStatus {
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

        Ok(WasmStoreOverviewResponse {
            publication,
            stores,
        })
    }

    // Return retired-store GC planning status for the current subnet, if any store is retired.
    pub async fn retired_publication_store_status()
    -> Result<Option<WasmStoreRetiredStoreStatusResponse>, Error> {
        WasmStorePublicationWorkflow::retired_publication_store_status()
            .await
            .map_err(Error::from)
    }

    // Mark the current retired publication store as prepared for store-local GC execution.
    pub async fn prepare_retired_publication_store_for_gc()
    -> Result<Option<WasmStoreBinding>, Error> {
        WasmStorePublicationWorkflow::prepare_retired_publication_store_for_gc()
            .await
            .map_err(Error::from)
    }

    // Mark the current retired publication store as actively executing store-local GC.
    pub async fn begin_retired_publication_store_gc() -> Result<Option<WasmStoreBinding>, Error> {
        WasmStorePublicationWorkflow::begin_retired_publication_store_gc()
            .await
            .map_err(Error::from)
    }

    // Mark the current retired publication store as having completed its local GC pass.
    pub async fn complete_retired_publication_store_gc() -> Result<Option<WasmStoreBinding>, Error>
    {
        WasmStorePublicationWorkflow::complete_retired_publication_store_gc()
            .await
            .map_err(Error::from)
    }

    // Clear the current retired publication binding after the local store GC run has completed.
    pub async fn finalize_retired_publication_store_binding()
    -> Result<Option<(WasmStoreBinding, Principal)>, Error> {
        WasmStorePublicationWorkflow::finalize_retired_publication_store_binding()
            .await
            .map_err(Error::from)
    }

    // Delete one finalized retired publication store after root publication state no longer references it.
    pub async fn delete_finalized_publication_store(
        binding: WasmStoreBinding,
        store_pid: Principal,
    ) -> Result<(), Error> {
        WasmStorePublicationWorkflow::delete_finalized_publication_store(binding, store_pid)
            .await
            .map_err(Error::from)
    }
}

///
/// WasmStoreApi
///

pub struct WasmStoreApi;

impl WasmStoreApi {
    fn current_store_limits() -> Result<WasmStoreLimits, Error> {
        let store = ConfigOps::current_wasm_store()?;

        Ok(WasmStoreLimits {
            max_store_bytes: store.max_store_bytes(),
            max_templates: store.max_templates(),
            max_template_versions_per_template: store.max_template_versions_per_template(),
        })
    }

    fn current_store_headroom_bytes() -> Result<Option<u64>, Error> {
        Ok(ConfigOps::current_wasm_store()?.headroom_bytes())
    }

    // Import the embedded template release set into this local store canister.
    pub fn import_embedded_release_set(wasms: &'static [(CanisterRole, &[u8])]) {
        WasmStorePublicationWorkflow::import_embedded_release_set_to_local_store(wasms);
    }

    // Return the approved template release catalog for this local store.
    pub fn template_catalog() -> Result<Vec<WasmStoreCatalogEntryResponse>, Error> {
        Ok(TemplateManifestOps::approved_catalog_response())
    }

    // Return current occupied-byte, retention, and store-local GC state for this local wasm store.
    pub fn template_status(gc: WasmStoreGcStatus) -> Result<WasmStoreStatusResponse, Error> {
        Ok(TemplateManifestOps::store_status_response(
            Self::current_store_limits()?,
            Self::current_store_headroom_bytes()?,
            gc,
        ))
    }

    // Prepare deterministic chunk-set metadata before chunk-by-chunk publication begins.
    pub fn prepare_chunk_set(
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        let now_secs = IcOps::now_secs();
        TemplateManifestOps::prepare_chunk_set_in_store_from_input(
            request,
            now_secs,
            Self::current_store_limits()?,
        )
        .map_err(Error::from)
    }

    // Publish one deterministic chunk into an already prepared local template release.
    pub fn publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
        TemplateManifestOps::publish_chunk_in_store_from_input(
            request,
            Self::current_store_limits()?,
        )
        .map_err(Error::from)
    }

    // Clear all local template metadata and chunk bytes for store-local GC execution.
    pub async fn execute_local_store_gc() -> Result<WasmStoreGcExecutionStats, Error> {
        TemplateManifestOps::execute_local_store_gc()
            .await
            .map_err(Error::from)
    }

    // Return deterministic chunk-set metadata for one local template release.
    pub fn template_info(
        template_id: TemplateId,
        version: TemplateVersion,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        TemplateManifestOps::chunk_set_info_response(&template_id, &version).map_err(Error::from)
    }

    // Return one deterministic chunk for one local template release.
    pub fn template_chunk(
        template_id: TemplateId,
        version: TemplateVersion,
        chunk_index: u32,
    ) -> Result<TemplateChunkResponse, Error> {
        TemplateManifestOps::chunk_response(&template_id, &version, chunk_index)
            .map_err(Error::from)
    }
}
