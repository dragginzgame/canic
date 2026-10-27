#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
use crate::{
    config,
    ids::WasmStoreGcStatus,
    ops::storage::template::{TemplateManifestOps, WasmStoreLimits},
};
#[cfg(feature = "wasm-store-canister")]
use crate::{
    dto::template::{
        TemplateChunkInput, TemplateChunkResponse, TemplateChunkSetInfoResponse,
        TemplateChunkSetPrepareInput, TemplateLookupRequest, TemplateManifestInput,
        TemplateStagingStatusResponse, WasmStoreCatalogEntryResponse,
        WasmStoreDeletionCycleReclamationRequest, WasmStoreDeletionCycleReclamationResponse,
        WasmStoreGcOperationStatus, WasmStoreGcRequest, WasmStoreStatusResponse,
    },
    ids::{TemplateId, TemplateVersion},
    ops::storage::template::{TemplateChunkedOps, WasmStoreGcOps},
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
use canic_contracts::dto::error::Error;
#[cfg(feature = "root-control-plane")]
use canic_contracts::dto::root_store::{RootStoreBootstrapRequest, RootStoreBootstrapResponse};
#[cfg(feature = "wasm-store-canister")]
use canic_core::control_plane_support::ops::ic::IcOps;

/// Admit Store mutations from the exact Root or retained exact installation controller.

#[cfg(feature = "wasm-store-canister")]
pub struct WasmStoreMutationCallerPredicate;

#[cfg(feature = "wasm-store-canister")]
impl canic_core::access::expr::AccessCheck for WasmStoreMutationCallerPredicate {
    fn eval(
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
        canic_core::access::auth::is_controller(caller)
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
    /// Observe a retained source without repeating its registration command.
    pub async fn root_fixture_status(
        content_id: [u8; 32],
    ) -> Result<
        Result<
            canic_contracts::dto::fixture_provisioning::FixtureSourceStatus,
            canic_contracts::dto::fixture_provisioning::FixtureStoreError,
        >,
        Error,
    > {
        crate::workflow::bootstrap::root_store::source_status(content_id)
            .await
            .map_err(Error::from)
    }

    /// Register a source selected only from the installed Root release manifest.
    pub async fn prepare_root_store_fixture(
        request: canic_contracts::dto::root_store::RootStoreFixturePrepareRequest,
    ) -> Result<
        Result<
            canic_contracts::dto::fixture_provisioning::FixtureSourceStatus,
            canic_contracts::dto::fixture_provisioning::FixtureStoreError,
        >,
        Error,
    > {
        crate::workflow::bootstrap::root_store::prepare(request)
            .await
            .map_err(Error::from)
    }

    /// Bootstrap the exact topology-admitted initial release set into this root's local Store.
    pub async fn bootstrap_root_store(
        request: RootStoreBootstrapRequest,
    ) -> Result<RootStoreBootstrapResponse, Error> {
        crate::workflow::bootstrap::root_store::bootstrap(request)
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
    /// Resolve exact garbage-collection evidence without mutation.
    pub fn gc_operation_status(
        operation_id: [u8; 32],
    ) -> Result<WasmStoreGcOperationStatus, Error> {
        if operation_id == [0; 32] {
            return Err(Error::from_registered(
                canic_contracts::diagnostics::codes::REQUEST_INVALID,
            ));
        }
        let gc = WasmStoreGcOps::status();
        if gc.operation_id != Some(operation_id) {
            return Err(Error::from_registered(
                canic_contracts::diagnostics::codes::STATE_UNAVAILABLE,
            ));
        }
        Ok(WasmStoreGcOperationStatus {
            operation_id,
            gc: crate::dto::template::WasmStoreGcStatusResponse {
                mode: gc.mode,
                changed_at: gc.changed_at,
                prepared_at: gc.prepared_at,
                started_at: gc.started_at,
                completed_at: gc.completed_at,
                runs_completed: gc.runs_completed,
            },
        })
    }

    /// Resolve the retained activation operation, preserving GC's existing selection priority.
    pub fn activation_operation_status(
        operation_id: [u8; 32],
    ) -> Result<canic_contracts::dto::fleet_activation::FleetActivationStatusResponse, Error> {
        if operation_id == [0; 32] {
            return Err(Error::from_registered(
                canic_contracts::diagnostics::codes::REQUEST_INVALID,
            ));
        }
        if WasmStoreGcOps::status().operation_id == Some(operation_id) {
            return Err(Error::from_registered(
                canic_contracts::diagnostics::codes::STATE_UNAVAILABLE,
            ));
        }
        let activation = canic_core::api::fleet_activation::FleetActivationApi::status()?;
        if activation.identity.operation_id != operation_id {
            return Err(Error::from_registered(
                canic_contracts::diagnostics::codes::STATE_UNAVAILABLE,
            ));
        }
        Ok(activation)
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

    /// Submit an explicit preparation or collection intent to the Store GC owner.
    pub fn request_gc(request: WasmStoreGcRequest) -> Result<(), Error> {
        crate::workflow::wasm_store::gc::request(request)
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
    TemplateChunkedOps::publish_chunk_in_store_from_input(request, now_secs(), limits)
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
        let errors = [
            WasmStoreCanisterApi::gc_operation_status([0; 32]).unwrap_err(),
            WasmStoreCanisterApi::activation_operation_status([0; 32]).unwrap_err(),
        ];
        for error in errors {
            assert_eq!(
                error.code(),
                canic_contracts::diagnostics::codes::REQUEST_INVALID.raw_code()
            );
        }
    }
}

#[cfg(all(test, feature = "root-control-plane"))]
mod root_tests {
    use super::publication_overview;
    use crate::{
        ids::{WasmStoreBinding, WasmStoreGcMode},
        ops::storage::state::root_wasm_store::RootWasmStoreStateOps,
        storage::stable::state::root_wasm_store::{
            RootWasmStoreState, RootWasmStoreStateData, SiblingWasmStoreAdoptionPhaseRecord,
            SiblingWasmStoreAdoptionRecord,
        },
    };
    use canic_core::cdk::types::Principal;

    #[test]
    fn store_overview_timestamps_use_epoch_seconds_and_preserve_unset_values() {
        RootWasmStoreState::import(RootWasmStoreStateData::default());
        let pid = Principal::from_slice(&[8; 29]);
        let binding = WasmStoreBinding::owned(pid.to_text());
        let created_at = 1_791_808_292;
        let adopted_at_ns = created_at * 1_000_000_000 + 418_714_000;
        RootWasmStoreState::begin_sibling_wasm_store_adoption(SiblingWasmStoreAdoptionRecord {
            operation_id: [1; 32],
            wasm_store: pid,
            expected_module_hash: [2; 32],
            controllers: vec![
                Principal::from_slice(&[9; 29]),
                Principal::from_slice(&[10; 29]),
            ],
            phase: SiblingWasmStoreAdoptionPhaseRecord::MutationInFlight,
            adopted_at_ns: None,
        })
        .unwrap();
        let receipt =
            RootWasmStoreState::commit_sibling_wasm_store_adoption([1; 32], adopted_at_ns).unwrap();
        assert_eq!(receipt.adopted_at_ns, Some(adopted_at_ns));
        let overview = publication_overview();
        assert_eq!(overview.stores[0].created_at, created_at);
        assert_eq!(overview.publication.changed_at, 0);
        assert_eq!(overview.publication.retired_at, 0);
        assert_eq!(overview.stores[0].gc.changed_at, 0);
        assert_eq!(overview.stores[0].gc.prepared_at, None);
        assert_eq!(overview.stores[0].gc.started_at, None);
        assert_eq!(overview.stores[0].gc.completed_at, None);

        assert!(RootWasmStoreStateOps::activate_publication_store_binding(
            binding.clone(),
            created_at + 1,
        ));
        for (mode, offset) in [
            (WasmStoreGcMode::Prepared, 2),
            (WasmStoreGcMode::InProgress, 3),
            (WasmStoreGcMode::Complete, 4),
        ] {
            assert!(RootWasmStoreStateOps::transition_wasm_store_gc(
                &binding,
                mode,
                created_at + offset,
            ));
        }
        let overview = publication_overview();
        assert_eq!(overview.publication.changed_at, created_at + 1);
        assert_eq!(overview.publication.retired_at, 0);
        let gc = &overview.stores[0].gc;
        assert_eq!(gc.changed_at, created_at + 4);
        assert_eq!(gc.prepared_at, Some(created_at + 2));
        assert_eq!(gc.started_at, Some(created_at + 3));
        assert_eq!(gc.completed_at, Some(created_at + 4));
        assert!(overview.stores[0].created_at < overview.publication.changed_at);
        assert!(overview.publication.changed_at < gc.prepared_at.unwrap());
        assert!(gc.prepared_at < gc.started_at);
        assert!(gc.started_at < gc.completed_at);

        assert!(RootWasmStoreStateOps::clear_publication_store_binding(
            created_at + 5
        ));
        assert_eq!(
            RootWasmStoreStateOps::retire_detached_publication_store_binding(created_at + 6),
            Some(binding),
        );
        let overview = publication_overview();
        assert_eq!(overview.publication.changed_at, created_at + 6);
        assert_eq!(overview.publication.retired_at, created_at + 6);
        assert!(overview.stores[0].gc.completed_at.unwrap() < overview.publication.retired_at);
        let replay = RootWasmStoreState::commit_sibling_wasm_store_adoption(
            [1; 32],
            adopted_at_ns + 10_000_000_000,
        )
        .unwrap();
        assert_eq!(replay, receipt);
        assert_eq!(publication_overview().stores[0].created_at, created_at);
        RootWasmStoreState::import(RootWasmStoreStateData::default());
    }
}

#[cfg(all(test, feature = "wasm-store-canister"))]
mod status_tests {
    use super::*;

    #[test]
    fn operation_reads_reject_zero_identity_before_state_lookup() {
        let invalid = Error::from_registered(canic_contracts::diagnostics::codes::REQUEST_INVALID);
        assert_eq!(
            WasmStoreCanisterApi::gc_operation_status([0; 32]).unwrap_err(),
            invalid
        );
        assert_eq!(
            WasmStoreCanisterApi::activation_operation_status([0; 32]).unwrap_err(),
            invalid
        );
    }
}
