pub use crate::ops::storage::template::WasmStoreGcExecutionStats;
pub use canic_core::api::runtime::install::ApprovedModuleSource;

use crate::{
    config,
    ids::{CanisterRole, TemplateId, TemplateVersion, WasmStoreBinding, WasmStoreGcStatus},
    ops::storage::{
        state::subnet::SubnetStateOps,
        template::{TemplateChunkedOps, TemplateManifestOps, WasmStoreLimits},
    },
    workflow::runtime::template::WasmStorePublicationWorkflow,
};
use canic_core::{__control_plane_core as cp_core, dto::error::Error};
use canic_template_types::dto::template::{
    TemplateChunkInput, TemplateChunkResponse, TemplateChunkSetInfoResponse,
    TemplateChunkSetPrepareInput, TemplateManifestInput, WasmStoreAdminCommand,
    WasmStoreAdminResponse, WasmStoreBootstrapDebugResponse, WasmStoreCatalogEntryResponse,
    WasmStoreOverviewResponse, WasmStorePublicationSlotResponse, WasmStoreStatusResponse,
};
use cp_core::{cdk::types::Principal, ops::ic::IcOps};

/// Return the current replica time in whole seconds.
#[must_use]
pub fn now_secs() -> u64 {
    IcOps::now_secs()
}

/// Stage one approved manifest in the current canister's local bootstrap source.
pub fn stage_manifest(input: TemplateManifestInput) {
    TemplateManifestOps::replace_approved_from_input(input);
}

/// Prepare one local chunk set for chunk-by-chunk staging in the current canister.
pub fn prepare_chunk_set(
    request: TemplateChunkSetPrepareInput,
) -> Result<TemplateChunkSetInfoResponse, Error> {
    TemplateChunkedOps::prepare_chunk_set_from_input(request, now_secs()).map_err(Error::from)
}

/// Stage one chunk into the current canister's local bootstrap source.
pub fn publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
    TemplateChunkedOps::publish_chunk_from_input(request).map_err(Error::from)
}

/// Resolve the currently approved module source for one role through the template-backed driver.
pub async fn approved_module_source_for_role(
    role: &CanisterRole,
) -> Result<ApprovedModuleSource, Error> {
    crate::workflow::runtime::template::resolved_approved_module_source_for_role(role)
        .await
        .map_err(Error::from)
}

/// Publish all root-local staged releases into the current subnet's selected wasm store.
pub async fn publish_staged_release_set_to_current_store() -> Result<(), Error> {
    WasmStorePublicationWorkflow::publish_staged_release_set_to_current_store()
        .await
        .map_err(Error::from)
}

/// Return root-owned staged bootstrap visibility for the bootstrap role and release buffer.
pub fn bootstrap_debug(
    bootstrap_role: &CanisterRole,
) -> Result<WasmStoreBootstrapDebugResponse, Error> {
    TemplateChunkedOps::bootstrap_debug_response(bootstrap_role).map_err(Error::from)
}

/// Execute one typed root-owned WasmStore publication or lifecycle admin command.
pub async fn publication_admin(
    cmd: WasmStoreAdminCommand,
) -> Result<WasmStoreAdminResponse, Error> {
    WasmStorePublicationWorkflow::handle_admin(cmd)
        .await
        .map_err(Error::from)
}

/// Publish the current release set into one subnet-local wasm store.
pub async fn publish_current_release_set_to_store(store_pid: Principal) -> Result<(), Error> {
    WasmStorePublicationWorkflow::publish_current_release_set_to_store(store_pid)
        .await
        .map_err(Error::from)
}

/// Publish the current release set into the current subnet's selected publication store.
pub async fn publish_current_release_set_to_current_store() -> Result<(), Error> {
    WasmStorePublicationWorkflow::publish_current_release_set_to_current_store()
        .await
        .map_err(Error::from)
}

/// Persist one explicit publication binding for the current subnet.
pub fn set_current_publication_store_binding(binding: WasmStoreBinding) -> Result<(), Error> {
    WasmStorePublicationWorkflow::set_current_publication_store_binding(binding)
        .map_err(Error::from)
}

/// Clear the explicit publication binding for the current subnet.
pub fn clear_current_publication_store_binding() {
    WasmStorePublicationWorkflow::clear_current_publication_store_binding();
}

/// Retire the current detached publication binding for the current subnet.
#[must_use]
pub fn retire_detached_publication_store_binding() -> Option<WasmStoreBinding> {
    WasmStorePublicationWorkflow::retire_detached_publication_store_binding()
}

/// Return the current root-owned overview for every tracked runtime-managed wasm store.
#[must_use]
pub fn publication_overview() -> WasmStoreOverviewResponse {
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

/// Mark the current retired publication store as prepared for store-local GC execution.
pub async fn prepare_retired_publication_store_for_gc() -> Result<Option<WasmStoreBinding>, Error> {
    WasmStorePublicationWorkflow::prepare_retired_publication_store_for_gc()
        .await
        .map_err(Error::from)
}

/// Mark the current retired publication store as actively executing store-local GC.
pub async fn begin_retired_publication_store_gc() -> Result<Option<WasmStoreBinding>, Error> {
    WasmStorePublicationWorkflow::begin_retired_publication_store_gc()
        .await
        .map_err(Error::from)
}

/// Mark the current retired publication store as having completed its local GC pass.
pub async fn complete_retired_publication_store_gc() -> Result<Option<WasmStoreBinding>, Error> {
    WasmStorePublicationWorkflow::complete_retired_publication_store_gc()
        .await
        .map_err(Error::from)
}

/// Finalize the retired publication binding once store-local GC has completed.
pub async fn finalize_retired_publication_store_binding() -> Result<Option<WasmStoreBinding>, Error>
{
    WasmStorePublicationWorkflow::finalize_retired_publication_store_binding()
        .await
        .map(|result| result.map(|(binding, _)| binding))
        .map_err(Error::from)
}

/// Delete one finalized runtime-managed publication store canister.
pub async fn delete_finalized_publication_store(
    binding: WasmStoreBinding,
    store_pid: Principal,
) -> Result<(), Error> {
    WasmStorePublicationWorkflow::delete_finalized_publication_store(binding, store_pid)
        .await
        .map_err(Error::from)
}

/// Return the current approved release catalog stored in this local wasm store.
#[must_use]
pub fn local_template_catalog() -> Vec<WasmStoreCatalogEntryResponse> {
    TemplateManifestOps::approved_catalog_response()
}

/// Return occupied-byte and retention state for this local wasm store.
pub fn local_template_status(gc: WasmStoreGcStatus) -> Result<WasmStoreStatusResponse, Error> {
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

/// Prepare one approved template release for chunk-by-chunk publication in this local store.
pub fn local_prepare_chunk_set(
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

/// Publish one deterministic chunk into an already prepared local template release.
pub fn local_publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
    let store = config::current_wasm_store().map_err(Error::from)?;
    let limits = WasmStoreLimits {
        max_store_bytes: store.max_store_bytes(),
        max_templates: store.max_templates(),
        max_template_versions_per_template: store.max_template_versions_per_template(),
    };
    TemplateChunkedOps::publish_chunk_in_store_from_input(request, limits).map_err(Error::from)
}

/// Clear all local template records, chunk metadata, and staged chunk hashes for GC.
pub async fn execute_local_store_gc() -> Result<WasmStoreGcExecutionStats, Error> {
    TemplateChunkedOps::execute_local_store_gc()
        .await
        .map_err(Error::from)
}

/// Return deterministic chunk-set metadata for one local template release.
pub fn local_template_info(
    template_id: TemplateId,
    version: TemplateVersion,
) -> Result<TemplateChunkSetInfoResponse, Error> {
    TemplateChunkedOps::chunk_set_info_response(&template_id, &version).map_err(Error::from)
}

/// Return one deterministic chunk for one local template release.
pub fn local_template_chunk(
    template_id: TemplateId,
    version: TemplateVersion,
    chunk_index: u32,
) -> Result<TemplateChunkResponse, Error> {
    TemplateChunkedOps::chunk_response(&template_id, &version, chunk_index).map_err(Error::from)
}
