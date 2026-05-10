use crate::{
    config,
    dto::template::TemplateManifestResponse,
    ids::WasmStoreBinding,
    ops::storage::template::TemplateChunkedOps,
    workflow::runtime::template::publication::{
        WasmStorePublicationWorkflow,
        fleet::{
            PublicationPlacement, PublicationPlacementAction, PublicationStoreFleet,
            PublicationStoreSnapshot,
        },
        store::{store_binding_for_pid, store_catalog, store_status},
    },
};
use canic_core::__control_plane_core as cp_core;
use canic_core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
};
use canic_core::{log, log::Topic};
use cp_core::{InternalError, InternalErrorOrigin, cdk::types::Principal, ops::ic::IcOps};

use super::super::super::WASM_STORE_BOOTSTRAP_BINDING;
use super::metrics::record_wasm_store_metric;

impl WasmStorePublicationWorkflow {
    // Resolve one automatic managed placement from the live fleet snapshot.
    async fn resolve_managed_publication_placement(
        fleet: &mut PublicationStoreFleet,
        manifest: &TemplateManifestResponse,
    ) -> Result<PublicationPlacement, InternalError> {
        if let Some(placement) = fleet.select_existing_store_for_release(manifest)? {
            return Ok(placement);
        }

        let store_config = config::current_subnet_default_wasm_store();
        if manifest.payload_size_bytes > store_config.max_store_bytes() {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "release {} exceeds empty wasm store capacity: bytes {} > {}",
                    Self::release_label(manifest),
                    manifest.payload_size_bytes,
                    store_config.max_store_bytes()
                ),
            ));
        }

        let created = Self::create_store_for_fleet(fleet).await?;
        let created_store = fleet
            .stores
            .iter()
            .find(|store| store.binding == created.binding)
            .ok_or_else(|| {
                InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!("new ws '{}' missing from fleet snapshot", created.binding),
                )
            })?;

        if !created_store.can_accept_release(manifest) {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "release {} does not fit empty store {}",
                    Self::release_label(manifest),
                    created.binding
                ),
            ));
        }

        Ok(created)
    }

    // Publish one approved manifest through the managed store fleet or reuse an exact existing release.
    async fn publish_manifest_to_managed_fleet(
        fleet: &mut PublicationStoreFleet,
        manifest: TemplateManifestResponse,
    ) -> Result<(), InternalError> {
        let release_label = Self::release_label(&manifest);
        let placement = Self::resolve_managed_publication_placement(fleet, &manifest).await?;

        match placement.action {
            PublicationPlacementAction::Reuse => {
                record_wasm_store_metric(
                    WasmStoreMetricOperation::ReleasePublish,
                    WasmStoreMetricSource::ManagedFleet,
                    WasmStoreMetricOutcome::Skipped,
                    WasmStoreMetricReason::CacheHit,
                );
                Self::mirror_manifest_to_root_state(placement.binding.clone(), &manifest);
                log!(
                    Topic::Wasm,
                    Info,
                    "ws reuse {} on {} ({})",
                    release_label,
                    placement.binding,
                    placement.pid
                );
            }
            PublicationPlacementAction::Publish | PublicationPlacementAction::Create => {
                Self::publish_manifest_to_placement(
                    fleet,
                    &placement,
                    &release_label,
                    manifest.clone(),
                )
                .await?;
            }
        }

        fleet.record_placement(&placement.binding, &manifest);
        Ok(())
    }

    async fn publish_manifest_to_placement(
        fleet: &mut PublicationStoreFleet,
        placement: &PublicationPlacement,
        release_label: &str,
        manifest: TemplateManifestResponse,
    ) -> Result<(), InternalError> {
        let action_label = if placement.action == PublicationPlacementAction::Create {
            "create"
        } else {
            "publish"
        };
        let store_index = fleet
            .store_index_for_binding(&placement.binding)
            .ok_or_else(|| missing_store_snapshot(&placement.binding))?;

        let publish_result = {
            let target_store = &mut fleet.stores[store_index];
            Self::publish_manifest_to_store(target_store, manifest.clone()).await
        };

        match publish_result {
            Ok(()) => {
                log!(
                    Topic::Wasm,
                    Info,
                    "ws place {} mode={} binding={} pid={}",
                    release_label,
                    action_label,
                    placement.binding,
                    placement.pid
                );
                Ok(())
            }
            Err(err) if Self::is_store_capacity_exceeded(&err) => {
                Self::rollover_and_publish_manifest(fleet, placement, release_label, manifest, err)
                    .await
            }
            Err(err) => Err(err),
        }
    }

    async fn rollover_and_publish_manifest(
        fleet: &mut PublicationStoreFleet,
        placement: &PublicationPlacement,
        release_label: &str,
        manifest: TemplateManifestResponse,
        err: InternalError,
    ) -> Result<(), InternalError> {
        record_wasm_store_metric(
            WasmStoreMetricOperation::ReleasePublish,
            WasmStoreMetricSource::ManagedFleet,
            WasmStoreMetricOutcome::Failed,
            WasmStoreMetricReason::Capacity,
        );
        if placement.action == PublicationPlacementAction::Create {
            return Err(err);
        }

        let retry = Self::create_store_for_fleet(fleet).await?;
        let retry_index = fleet
            .store_index_for_binding(&retry.binding)
            .ok_or_else(|| missing_store_snapshot(&retry.binding))?;
        {
            let target_store = &mut fleet.stores[retry_index];
            Self::publish_manifest_to_store(target_store, manifest.clone()).await?;
        }
        record_wasm_store_metric(
            WasmStoreMetricOperation::ReleasePublish,
            WasmStoreMetricSource::ManagedFleet,
            WasmStoreMetricOutcome::Completed,
            WasmStoreMetricReason::Capacity,
        );
        log!(
            Topic::Wasm,
            Warn,
            "ws rollover {} from {} to {}",
            release_label,
            placement.binding,
            retry.binding
        );
        fleet.record_placement(&retry.binding, &manifest);
        Ok(())
    }

    // Publish all root-local staged releases into the current subnet's selected wasm store.
    pub async fn publish_staged_release_set_to_current_store() -> Result<(), InternalError> {
        let manifests = Self::managed_release_manifests()?
            .into_iter()
            .filter(|manifest| manifest.store_binding == WASM_STORE_BOOTSTRAP_BINDING)
            .collect::<Vec<_>>();

        for manifest in &manifests {
            TemplateChunkedOps::validate_staged_release(manifest)?;
        }

        let mut fleet = Self::snapshot_publication_store_fleet().await?;
        for manifest in manifests {
            Self::publish_manifest_to_managed_fleet(&mut fleet, manifest).await?;
        }

        Ok(())
    }

    // Publish the current release set from the current default store into one subnet-local wasm store.
    pub async fn publish_current_release_set_to_store(
        target_store_pid: Principal,
    ) -> Result<(), InternalError> {
        let target_store_binding = store_binding_for_pid(target_store_pid)?;
        let target_status = store_status(target_store_pid).await?;
        let target_catalog = store_catalog(target_store_pid).await?;
        let mut target_store = PublicationStoreSnapshot {
            binding: target_store_binding.clone(),
            pid: target_store_pid,
            created_at: IcOps::now_secs(),
            status: target_status,
            releases: target_catalog,
            stored_chunk_hashes: None,
        };

        for manifest in Self::managed_release_manifests()? {
            if target_store.has_exact_release(&manifest) {
                Self::mirror_manifest_to_root_state(target_store_binding.clone(), &manifest);
                continue;
            }

            if !target_store.can_accept_release(&manifest) {
                return Err(InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!(
                        "target ws '{}' cannot fit {}",
                        target_store_binding,
                        Self::release_label(&manifest)
                    ),
                ));
            }

            Self::publish_manifest_to_store(&mut target_store, manifest.clone()).await?;
            target_store.record_release(&manifest);
        }

        Ok(())
    }

    /// Publish the current managed release set into the managed subnet-local store fleet.
    pub async fn publish_current_release_set_to_current_store() -> Result<(), InternalError> {
        let mut fleet = Self::snapshot_publication_store_fleet().await?;

        for manifest in Self::managed_release_manifests()? {
            Self::publish_manifest_to_managed_fleet(&mut fleet, manifest).await?;
        }

        Ok(())
    }
}

fn missing_store_snapshot(binding: &WasmStoreBinding) -> InternalError {
    InternalError::workflow(
        InternalErrorOrigin::Workflow,
        format!("ws '{binding}' missing from fleet snapshot"),
    )
}
