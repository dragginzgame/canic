#[cfg(feature = "wasm-store-canister")]
use crate::dto::template::{
    TemplateChunkInput, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
    TemplateManifestInput,
};
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
use crate::{
    config,
    ids::WasmStoreGcStatus,
    ops::storage::template::{TemplateManifestOps, WasmStoreLimits},
};
#[cfg(feature = "wasm-store-canister")]
use crate::{
    dto::template::{
        StoreOperationStatusResponse, TemplateChunkResponse, TemplateLookupRequest,
        TemplateStagingStatusResponse, WasmStoreCatalogEntryResponse,
        WasmStoreDeletionCycleReclamationRequest, WasmStoreDeletionCycleReclamationResponse,
        WasmStoreGcOperationStatus, WasmStoreStatusResponse,
    },
    ids::{TemplateId, TemplateVersion, WasmStoreGcMode},
    ops::storage::template::{TemplateChunkedOps, WasmStoreGcExecutionStats, WasmStoreGcOps},
};
#[cfg(feature = "root-control-plane")]
use crate::{
    dto::template::{
        WasmStoreAdminCommand, WasmStoreAdminResponse, WasmStoreOverviewResponse,
        WasmStorePublicationSlotResponse,
    },
    ops::storage::state::root_wasm_store::RootWasmStoreStateOps,
    workflow::runtime::template::WasmStorePublicationWorkflow,
};
#[cfg(feature = "wasm-store-canister")]
use async_trait::async_trait;
#[cfg(feature = "wasm-store-canister")]
use canic_core::control_plane_support::ops::ic::IcOps;
use canic_core::dto::error::Error;
#[cfg(feature = "root-control-plane")]
use canic_core::dto::root_store::{RootStoreBootstrapRequest, RootStoreBootstrapResponse};
#[cfg(feature = "wasm-store-canister")]
use canic_core::{log, log::Topic};

/// Admit Store mutations from the exact Root or retained exact installation controller.
#[cfg(feature = "wasm-store-canister")]
pub struct WasmStoreMutationCallerPredicate;

#[cfg(feature = "wasm-store-canister")]
#[async_trait]
impl canic_core::access::expr::AsyncAccessPredicate for WasmStoreMutationCallerPredicate {
    async fn eval(
        &self,
        ctx: &canic_core::access::expr::AccessContext,
    ) -> Result<(), canic_core::access::AccessError> {
        let authority = canic_core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::wasm_store_authority()
            .map_err(canic_core::access::AccessError::Internal)?;
        let caller = ctx.transport_caller();
        if caller == authority.fleet_subnet_root {
            return Ok(());
        }
        if caller != authority.installation_controller {
            return Err(canic_core::access::AccessError::RootRequired);
        }
        canic_core::access::auth::is_controller(caller).await
    }

    fn name(&self) -> &'static str {
        "caller_is_root_or_retained_installation_controller"
    }
}

///
/// WasmStoreBootstrapApi
///

#[cfg(feature = "root-control-plane")]
pub struct WasmStoreBootstrapApi;

#[cfg(feature = "root-control-plane")]
impl WasmStoreBootstrapApi {
    /// Bootstrap the exact topology-admitted initial release set into this root's local Store.
    pub async fn bootstrap_root_store(
        request: RootStoreBootstrapRequest,
    ) -> Result<RootStoreBootstrapResponse, Error> {
        crate::workflow::bootstrap::root_store::bootstrap(request)
            .await
            .map_err(Error::from)
    }

    /// Verify this root's exact live initial Store evidence without mutation.
    pub async fn root_store_status(
        request: RootStoreBootstrapRequest,
    ) -> Result<RootStoreBootstrapResponse, Error> {
        crate::workflow::bootstrap::root_store::status(request)
            .await
            .map_err(Error::from)
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
    /// Resolve the current Store-local operation identity without mutation.
    pub fn operation_status(operation_id: [u8; 32]) -> Result<StoreOperationStatusResponse, Error> {
        if operation_id == [0; 32] {
            return Err(Error::from_registered(
                canic_core::diagnostics::codes::REQUEST_INVALID,
            ));
        }
        let gc = WasmStoreGcOps::status();
        if gc.operation_id == Some(operation_id) {
            return Ok(StoreOperationStatusResponse::GarbageCollection(
                WasmStoreGcOperationStatus {
                    operation_id,
                    gc: crate::dto::template::WasmStoreGcStatusResponse {
                        mode: gc.mode,
                        changed_at: gc.changed_at,
                        prepared_at: gc.prepared_at,
                        started_at: gc.started_at,
                        completed_at: gc.completed_at,
                        runs_completed: gc.runs_completed,
                    },
                },
            ));
        }
        let activation = canic_core::api::fleet_activation::FleetActivationApi::status()?;
        if activation.identity.operation_id == operation_id {
            return Ok(StoreOperationStatusResponse::FleetActivation(activation));
        }
        Err(Error::from_registered(
            canic_core::diagnostics::codes::STATE_UNAVAILABLE,
        ))
    }

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

    // Return exact manifest and staged-chunk evidence for one release.
    pub fn staging_status(
        request: TemplateLookupRequest,
    ) -> Result<TemplateStagingStatusResponse, Error> {
        Ok(TemplateChunkedOps::staging_status_response(
            &request.template_id,
            &request.version,
        ))
    }

    // Return occupied-byte and retention state for this local wasm store.
    pub fn status() -> Result<WasmStoreStatusResponse, Error> {
        local_template_status(WasmStoreGcOps::snapshot())
    }

    // Mark this local wasm store as prepared for store-local GC execution.
    pub fn prepare_gc(operation_id: [u8; 32]) -> Result<(), Error> {
        WasmStoreGcOps::prepare(operation_id, now_secs())
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
            return Err(Error::from_registered(
                canic_core::diagnostics::codes::STATE_CONFLICT,
            ));
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

    /// Resume the exact Store-local GC operation through every private phase.
    pub async fn run_gc(operation_id: [u8; 32]) -> Result<(), Error> {
        let current = WasmStoreGcOps::status();
        if current.operation_id != Some(operation_id) {
            return Err(Error::from_registered(
                canic_core::diagnostics::codes::STATE_CONFLICT,
            ));
        }
        match current.mode {
            WasmStoreGcMode::Normal => {
                Self::prepare_gc(operation_id)?;
                Self::begin_gc()?;
                Self::complete_gc().await
            }
            WasmStoreGcMode::Prepared => {
                Self::begin_gc()?;
                Self::complete_gc().await
            }
            WasmStoreGcMode::InProgress | WasmStoreGcMode::Clearing => Self::complete_gc().await,
            WasmStoreGcMode::Complete => Ok(()),
        }
    }

    // Return transferable cycles to the authenticated root before physical Store deletion.
    pub async fn reclaim_deletion_cycles(
        request: WasmStoreDeletionCycleReclamationRequest,
    ) -> Result<WasmStoreDeletionCycleReclamationResponse, Error> {
        crate::workflow::wasm_store::reclaim_deletion_cycles(request)
            .await
            .map_err(Error::from)
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

#[cfg(feature = "wasm-store-canister")]
fn now_secs() -> u64 {
    IcOps::now_secs()
}

#[cfg(feature = "root-control-plane")]
async fn publication_admin(cmd: WasmStoreAdminCommand) -> Result<WasmStoreAdminResponse, Error> {
    WasmStorePublicationWorkflow::handle_admin(cmd)
        .await
        .map_err(Error::from)
}

#[cfg(feature = "root-control-plane")]
fn publication_overview() -> WasmStoreOverviewResponse {
    let store = config::fleet_subnet_root_default_wasm_store();
    let limits = WasmStoreLimits::from(&store);
    let headroom_bytes = store.headroom_bytes();
    let publication = RootWasmStoreStateOps::publication_store_state_response();
    let stores = RootWasmStoreStateOps::wasm_stores()
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
    let limits = WasmStoreLimits::from(&store);
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
    WasmStoreGcOps::require_writable()?;
    let store = config::current_wasm_store().map_err(Error::from)?;
    let limits = WasmStoreLimits::from(&store);
    TemplateChunkedOps::prepare_chunk_set_in_store_from_input(request, now_secs(), limits)
        .map_err(Error::from)
}

#[cfg(feature = "wasm-store-canister")]
fn local_stage_manifest(request: TemplateManifestInput) -> Result<(), Error> {
    WasmStoreGcOps::require_writable()?;
    let store = config::current_wasm_store().map_err(Error::from)?;
    let limits = WasmStoreLimits::from(&store);
    TemplateChunkedOps::replace_approved_in_store_from_input(request, limits).map_err(Error::from)
}

#[cfg(feature = "wasm-store-canister")]
fn local_publish_chunk(request: TemplateChunkInput) -> Result<(), Error> {
    WasmStoreGcOps::require_writable()?;
    let store = config::current_wasm_store().map_err(Error::from)?;
    let limits = WasmStoreLimits::from(&store);
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

#[cfg(all(test, feature = "wasm-store-canister"))]
mod tests {
    use super::WasmStoreCanisterApi;

    #[test]
    fn store_operation_status_rejects_the_zero_identity_before_state_access() {
        let Err(error) = WasmStoreCanisterApi::operation_status([0; 32]) else {
            panic!("the zero operation identity must be rejected");
        };

        assert_eq!(
            error.code(),
            canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
        );
    }
}
