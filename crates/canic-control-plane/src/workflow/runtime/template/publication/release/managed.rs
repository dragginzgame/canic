use crate::ops::storage::state::root_wasm_store::RootWasmStoreStateOps;
use crate::{
    dto::template::{TemplateManifestResponse, WasmStoreCatalogEntryResponse},
    ops::storage::template::TemplateChunkedOps,
    workflow::runtime::template::{
        publication::{
            WasmStorePublicationWorkflow,
            cost_guard::{PUBLICATION_BOOTSTRAP_COMMAND_KIND, PublicationCostGuard},
            error::PublicationWorkflowError,
            snapshot::PublicationStoreSnapshot,
            store::store_catalog,
        },
        record_wasm_store_metric,
    },
};
use canic_core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
};
use canic_core::cdk::types::Principal;
use canic_core::control_plane_support::{error::InternalError, ops::cost_guard::CostGuardPermit};
use canic_core::{log, log::Topic};

impl WasmStorePublicationWorkflow {
    /// Read the live catalog from the one root-local Store without mutating publication state.
    pub async fn single_store_catalog()
    -> Result<(Principal, Vec<WasmStoreCatalogEntryResponse>), InternalError> {
        let stores = RootWasmStoreStateOps::wasm_stores();
        if stores.len() != 1 {
            return Err(PublicationWorkflowError::SingleAdoptedStoreRequired {
                observed_count: stores.len(),
            }
            .into());
        }
        let store_pid = stores[0].pid;
        Ok((store_pid, store_catalog(store_pid).await?))
    }

    /// Publish one exact initial root release set into exactly one local Store.
    pub async fn bootstrap_exact_staged_release_set(
        manifests: Vec<TemplateManifestResponse>,
    ) -> Result<(Principal, Vec<WasmStoreCatalogEntryResponse>), InternalError> {
        let cost_guard = PublicationCostGuard::reserve(PUBLICATION_BOOTSTRAP_COMMAND_KIND)?;
        let result =
            Self::bootstrap_exact_staged_release_set_with_permit(manifests, cost_guard.permit())
                .await;
        cost_guard.settle(result)
    }

    async fn bootstrap_exact_staged_release_set_with_permit(
        manifests: Vec<TemplateManifestResponse>,
        publication_permit: &CostGuardPermit,
    ) -> Result<(Principal, Vec<WasmStoreCatalogEntryResponse>), InternalError> {
        for manifest in &manifests {
            TemplateChunkedOps::validate_staged_release(manifest)?;
        }

        let mut store = Self::snapshot_adopted_wasm_store(publication_permit).await?;
        Self::pin_initial_publication_store(store.binding.clone())?;
        let store_pid = store.pid;

        for manifest in manifests {
            Self::publish_manifest_to_adopted_store(&mut store, manifest, publication_permit)
                .await?;
        }

        let catalog = store_catalog(store_pid).await?;
        Ok((store_pid, catalog))
    }

    // Publish one approved manifest to the exact adopted Store or reuse its exact release.
    async fn publish_manifest_to_adopted_store(
        store: &mut PublicationStoreSnapshot,
        manifest: TemplateManifestResponse,
        publication_permit: &CostGuardPermit,
    ) -> Result<(), InternalError> {
        let release_label = Self::release_label(&manifest);
        Self::require_active_publication_store(&store.binding)?;
        if store.has_exact_release(&manifest) {
            record_wasm_store_metric(
                WasmStoreMetricOperation::ReleasePublish,
                WasmStoreMetricSource::ManagedFleet,
                WasmStoreMetricOutcome::Skipped,
                WasmStoreMetricReason::CacheHit,
            );
            Self::mirror_manifest_to_root_state(
                publication_permit,
                store.binding.clone(),
                &manifest,
            );
            log!(
                Topic::Wasm,
                Info,
                "ws reuse {} on {} ({})",
                release_label,
                store.binding,
                store.pid
            );
            return Ok(());
        }
        if let Some(conflict) = store.conflicting_release(&manifest) {
            return Err(PublicationWorkflowError::ReleaseConflict {
                template_id: manifest.template_id,
                version: manifest.version,
                binding: store.binding.clone(),
                existing_payload_hash: conflict.payload_hash.clone(),
                existing_payload_size_bytes: conflict.payload_size_bytes,
            }
            .into());
        }
        if !store.can_accept_release(&manifest) {
            return Err(PublicationWorkflowError::CapacityExceeded {
                release: release_label,
                target: store.binding.to_string(),
                payload_size_bytes: manifest.payload_size_bytes,
                remaining_store_bytes: store.status.remaining_store_bytes,
            }
            .into());
        }

        Self::publish_manifest_to_store(store, manifest.clone(), publication_permit).await?;
        store.record_release(&manifest);
        log!(
            Topic::Wasm,
            Info,
            "ws place {} mode=publish binding={} pid={}",
            release_label,
            store.binding,
            store.pid
        );
        Ok(())
    }

    /// Publish the current managed release set into the one adopted sibling Store.
    pub(in crate::workflow::runtime::template::publication) async fn publish_active_release_set_to_adopted_store(
        publication_permit: &CostGuardPermit,
    ) -> Result<(), InternalError> {
        let mut store = Self::snapshot_adopted_wasm_store(publication_permit).await?;
        Self::require_active_publication_store(&store.binding)?;

        for manifest in Self::managed_release_manifests()? {
            Self::publish_manifest_to_adopted_store(&mut store, manifest, publication_permit)
                .await?;
        }

        Ok(())
    }
}
