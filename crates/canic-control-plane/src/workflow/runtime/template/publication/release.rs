use super::{
    WasmStorePublicationWorkflow,
    fleet::{
        PublicationPlacement, PublicationPlacementAction, PublicationStoreFleet,
        PublicationStoreSnapshot,
    },
    store::{
        TemplateChunkInputRef, local_chunk, store_binding_for_pid, store_catalog, store_chunk,
        store_chunk_set_info, store_stage_manifest, store_status,
    },
};
use crate::{
    config,
    dto::template::{
        TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput, TemplateManifestInput,
        TemplateManifestResponse,
    },
    ids::{TemplateChunkingMode, TemplateManifestState, WasmStoreBinding},
    ops::storage::template::{TemplateChunkedOps, TemplateManifestOps},
};
use canic_core::__control_plane_core as cp_core;
use canic_core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
    WasmStoreMetricsApi,
};
use canic_core::{log, log::Topic};
use cp_core::{InternalError, InternalErrorOrigin, cdk::types::Principal, ops::ic::IcOps};

use super::super::{WASM_STORE_BOOTSTRAP_BINDING, store_pid_for_binding};

impl WasmStorePublicationWorkflow {
    // Return the deterministic approved manifests that still belong to the configured managed fleet.
    pub(super) fn managed_release_manifests() -> Result<Vec<TemplateManifestResponse>, InternalError>
    {
        let roles = config::current_subnet_managed_release_roles()?;

        Ok(
            TemplateManifestOps::approved_manifests_for_roles_response(&roles)
                .into_iter()
                .filter(|manifest| manifest.chunking_mode == TemplateChunkingMode::Chunked)
                .collect(),
        )
    }

    // Deprecate any currently approved managed release that no longer belongs to the configured fleet.
    pub fn prune_unconfigured_managed_releases() -> Result<usize, InternalError> {
        let roles = config::current_subnet_managed_release_roles()?;
        Ok(TemplateManifestOps::deprecate_approved_roles_not_in(&roles))
    }

    // Return the exact fleet stores that already carry one approved release.
    fn exact_release_candidates<'a>(
        fleet: &'a PublicationStoreFleet,
        manifest: &TemplateManifestResponse,
    ) -> Vec<&'a PublicationStoreSnapshot> {
        let mut stores = fleet
            .stores
            .iter()
            .filter(|store| store.has_exact_release(manifest))
            .collect::<Vec<_>>();

        stores.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then(left.binding.cmp(&right.binding))
        });

        stores
    }

    // Reconcile the approved release for one role against the exact matching fleet entries.
    pub(super) fn reconciled_binding_for_manifest(
        fleet: &PublicationStoreFleet,
        manifest: &TemplateManifestResponse,
    ) -> Result<WasmStoreBinding, InternalError> {
        let candidates = Self::exact_release_candidates(fleet, manifest);

        if candidates.is_empty() {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "fleet import missing exact release for role '{}': expected {}@{} on {}",
                    manifest.role, manifest.template_id, manifest.version, manifest.store_binding
                ),
            ));
        }

        if candidates
            .iter()
            .any(|store| store.binding == manifest.store_binding)
        {
            return Ok(manifest.store_binding.clone());
        }

        if let Some(binding) = fleet.preferred_binding.as_ref()
            && candidates.iter().any(|store| &store.binding == binding)
        {
            return Ok(binding.clone());
        }

        Ok(candidates[0].binding.clone())
    }

    // Build the source label used in placement logs for one approved manifest.
    fn release_label(manifest: &TemplateManifestResponse) -> String {
        format!("{}@{}", manifest.template_id, manifest.version)
    }

    // Resolve the source store pid for one manifest-backed release, if it is store-backed.
    fn source_store_pid_for_manifest(
        manifest: &TemplateManifestResponse,
    ) -> Result<Option<Principal>, InternalError> {
        if manifest.store_binding == WASM_STORE_BOOTSTRAP_BINDING {
            Ok(None)
        } else {
            store_pid_for_binding(&manifest.store_binding).map(Some)
        }
    }

    // Resolve deterministic chunk-set metadata for one manifest from its authoritative source.
    async fn source_chunk_set_info_for_manifest(
        manifest: &TemplateManifestResponse,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        match Self::source_store_pid_for_manifest(manifest)? {
            Some(store_pid) => {
                store_chunk_set_info(store_pid, &manifest.template_id, &manifest.version).await
            }
            None => TemplateChunkedOps::chunk_set_info_response(
                &manifest.template_id,
                &manifest.version,
            ),
        }
    }

    // Resolve one deterministic chunk for one manifest from its authoritative source.
    async fn source_chunk_for_manifest(
        manifest: &TemplateManifestResponse,
        chunk_index: u32,
    ) -> Result<Vec<u8>, InternalError> {
        match Self::source_store_pid_for_manifest(manifest)? {
            Some(store_pid) => {
                store_chunk(
                    store_pid,
                    &manifest.template_id,
                    &manifest.version,
                    chunk_index,
                )
                .await
            }
            None => local_chunk(&manifest.template_id, &manifest.version, chunk_index),
        }
    }

    // Return true when one failed store call represents store-capacity exhaustion.
    fn is_store_capacity_exceeded(err: &InternalError) -> bool {
        err.public_error().is_some_and(|public| {
            public
                .message
                .contains(Self::WASM_STORE_CAPACITY_EXCEEDED_MESSAGE)
        }) || err
            .to_string()
            .contains(Self::WASM_STORE_CAPACITY_EXCEEDED_MESSAGE)
    }

    // Mirror one approved manifest into root-owned state without mutating a live store.
    fn mirror_manifest_to_root_state(
        target_store_binding: WasmStoreBinding,
        manifest: &TemplateManifestResponse,
    ) {
        TemplateManifestOps::replace_approved_from_input(TemplateManifestInput {
            template_id: manifest.template_id.clone(),
            role: manifest.role.clone(),
            version: manifest.version.clone(),
            payload_hash: manifest.payload_hash.clone(),
            payload_size_bytes: manifest.payload_size_bytes,
            store_binding: target_store_binding,
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(IcOps::now_secs()),
            created_at: manifest.created_at,
        });
    }

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

    // Publish one approved manifest into the target store from its authoritative source.
    async fn publish_manifest_to_store(
        target_store: &mut PublicationStoreSnapshot,
        manifest: TemplateManifestResponse,
    ) -> Result<(), InternalError> {
        record_wasm_store_metric(
            WasmStoreMetricOperation::ReleasePublish,
            WasmStoreMetricSource::TargetStore,
            WasmStoreMetricOutcome::Started,
            WasmStoreMetricReason::Ok,
        );
        let chunk_hashes = Self::release_chunk_hashes(&manifest).await?;

        target_store.ensure_stored_chunk_hashes().await?;
        Self::prepare_target_store_for_manifest(target_store.pid, &manifest, &chunk_hashes).await?;
        Self::publish_manifest_chunks_to_store(target_store, &manifest, &chunk_hashes).await?;
        Self::promote_manifest_to_store_with_metrics(target_store, manifest.clone()).await?;

        log!(
            Topic::Wasm,
            Ok,
            "tpl.publish {} -> {}@{} (store={}, chunks={})",
            manifest.role,
            manifest.template_id,
            manifest.version,
            target_store.pid,
            chunk_hashes.len()
        );

        record_wasm_store_metric(
            WasmStoreMetricOperation::ReleasePublish,
            WasmStoreMetricSource::TargetStore,
            WasmStoreMetricOutcome::Completed,
            WasmStoreMetricReason::Ok,
        );

        Ok(())
    }

    // Resolve source chunk hashes and record release-level failure if lookup fails.
    async fn release_chunk_hashes(
        manifest: &TemplateManifestResponse,
    ) -> Result<Vec<Vec<u8>>, InternalError> {
        match Self::source_chunk_set_info_for_manifest(manifest).await {
            Ok(info) => Ok(info.chunk_hashes),
            Err(err) => {
                record_wasm_store_publish_failed(WasmStoreMetricReason::from_publication_error(
                    &err,
                ));
                Err(err)
            }
        }
    }

    // Prepare the target store for one manifest's canonical chunk set.
    async fn prepare_target_store_for_manifest(
        target_store_pid: Principal,
        manifest: &TemplateManifestResponse,
        chunk_hashes: &[Vec<u8>],
    ) -> Result<(), InternalError> {
        record_wasm_store_metric(
            WasmStoreMetricOperation::Prepare,
            WasmStoreMetricSource::TargetStore,
            WasmStoreMetricOutcome::Started,
            WasmStoreMetricReason::Ok,
        );

        let result: Result<TemplateChunkSetInfoResponse, InternalError> =
            super::super::call_store_result(
                target_store_pid,
                cp_core::protocol::CANIC_WASM_STORE_PREPARE,
                (TemplateChunkSetPrepareInput {
                    template_id: manifest.template_id.clone(),
                    version: manifest.version.clone(),
                    payload_hash: manifest.payload_hash.clone(),
                    payload_size_bytes: manifest.payload_size_bytes,
                    chunk_hashes: chunk_hashes.to_vec(),
                },),
            )
            .await;

        match result {
            Ok(_) => {
                record_wasm_store_metric(
                    WasmStoreMetricOperation::Prepare,
                    WasmStoreMetricSource::TargetStore,
                    WasmStoreMetricOutcome::Completed,
                    WasmStoreMetricReason::Ok,
                );
                canic_core::perf!("publish_prepare_store");
                Ok(())
            }
            Err(err) => {
                let reason = WasmStoreMetricReason::from_publication_error(&err);
                record_wasm_store_metric(
                    WasmStoreMetricOperation::Prepare,
                    WasmStoreMetricSource::TargetStore,
                    WasmStoreMetricOutcome::Failed,
                    reason,
                );
                record_wasm_store_publish_failed(reason);
                Err(err)
            }
        }
    }

    // Publish every source chunk to the target store and refresh install-cache chunks.
    async fn publish_manifest_chunks_to_store(
        target_store: &mut PublicationStoreSnapshot,
        manifest: &TemplateManifestResponse,
        chunk_hashes: &[Vec<u8>],
    ) -> Result<(), InternalError> {
        for (chunk_index, expected_hash) in chunk_hashes.iter().cloned().enumerate() {
            let chunk_index = u32::try_from(chunk_index).map_err(|_| {
                InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!(
                        "template '{}' exceeds chunk index bounds",
                        manifest.template_id
                    ),
                )
            })?;
            Self::publish_manifest_chunk_to_store(
                target_store,
                manifest,
                chunk_index,
                expected_hash,
            )
            .await?;
        }

        Ok(())
    }

    // Publish one source chunk to the target store and ensure install-cache availability.
    async fn publish_manifest_chunk_to_store(
        target_store: &mut PublicationStoreSnapshot,
        manifest: &TemplateManifestResponse,
        chunk_index: u32,
        expected_hash: Vec<u8>,
    ) -> Result<(), InternalError> {
        let already_uploaded = target_store
            .stored_chunk_hashes
            .as_ref()
            .is_some_and(|hashes| hashes.contains(&expected_hash));
        let bytes = Self::source_chunk_for_manifest_with_metrics(manifest, chunk_index).await?;

        Self::publish_chunk_to_target_store(target_store.pid, manifest, chunk_index, &bytes)
            .await?;
        Self::ensure_target_store_upload_cache(
            target_store,
            manifest,
            chunk_index,
            expected_hash,
            bytes,
            already_uploaded,
        )
        .await
    }

    // Resolve one source chunk and record publication failure metrics when lookup fails.
    async fn source_chunk_for_manifest_with_metrics(
        manifest: &TemplateManifestResponse,
        chunk_index: u32,
    ) -> Result<Vec<u8>, InternalError> {
        match Self::source_chunk_for_manifest(manifest, chunk_index).await {
            Ok(bytes) => Ok(bytes),
            Err(err) => {
                let reason = WasmStoreMetricReason::from_publication_error(&err);
                record_wasm_store_metric(
                    WasmStoreMetricOperation::ChunkPublish,
                    WasmStoreMetricSource::TargetStore,
                    WasmStoreMetricOutcome::Failed,
                    reason,
                );
                record_wasm_store_publish_failed(reason);
                Err(err)
            }
        }
    }

    // Push one chunk through the target store API.
    async fn publish_chunk_to_target_store(
        target_store_pid: Principal,
        manifest: &TemplateManifestResponse,
        chunk_index: u32,
        bytes: &[u8],
    ) -> Result<(), InternalError> {
        record_wasm_store_metric(
            WasmStoreMetricOperation::ChunkPublish,
            WasmStoreMetricSource::TargetStore,
            WasmStoreMetricOutcome::Started,
            WasmStoreMetricReason::Ok,
        );

        if let Err(err) = super::super::call_store_result::<(), _>(
            target_store_pid,
            cp_core::protocol::CANIC_WASM_STORE_PUBLISH_CHUNK,
            (TemplateChunkInputRef {
                template_id: &manifest.template_id,
                version: &manifest.version,
                chunk_index,
                bytes,
            },),
        )
        .await
        {
            let reason = WasmStoreMetricReason::from_publication_error(&err);
            record_wasm_store_metric(
                WasmStoreMetricOperation::ChunkPublish,
                WasmStoreMetricSource::TargetStore,
                WasmStoreMetricOutcome::Failed,
                reason,
            );
            record_wasm_store_publish_failed(reason);
            return Err(err);
        }

        record_wasm_store_metric(
            WasmStoreMetricOperation::ChunkPublish,
            WasmStoreMetricSource::TargetStore,
            WasmStoreMetricOutcome::Completed,
            WasmStoreMetricReason::Ok,
        );
        canic_core::perf!("publish_push_store_chunk");
        Ok(())
    }

    // Ensure the target store's management chunk cache contains one published chunk.
    async fn ensure_target_store_upload_cache(
        target_store: &mut PublicationStoreSnapshot,
        manifest: &TemplateManifestResponse,
        chunk_index: u32,
        expected_hash: Vec<u8>,
        bytes: Vec<u8>,
        already_uploaded: bool,
    ) -> Result<(), InternalError> {
        if already_uploaded {
            record_wasm_store_metric(
                WasmStoreMetricOperation::ChunkUpload,
                WasmStoreMetricSource::TargetStore,
                WasmStoreMetricOutcome::Skipped,
                WasmStoreMetricReason::CacheHit,
            );
            return Ok(());
        }

        record_wasm_store_metric(
            WasmStoreMetricOperation::ChunkUpload,
            WasmStoreMetricSource::TargetStore,
            WasmStoreMetricOutcome::Started,
            WasmStoreMetricReason::CacheMiss,
        );
        let uploaded_hash =
            match cp_core::ops::ic::mgmt::MgmtOps::upload_chunk(target_store.pid, bytes).await {
                Ok(uploaded_hash) => uploaded_hash,
                Err(err) => {
                    record_wasm_store_metric(
                        WasmStoreMetricOperation::ChunkUpload,
                        WasmStoreMetricSource::TargetStore,
                        WasmStoreMetricOutcome::Failed,
                        WasmStoreMetricReason::ManagementCall,
                    );
                    record_wasm_store_publish_failed(WasmStoreMetricReason::ManagementCall);
                    return Err(err);
                }
            };

        if uploaded_hash != expected_hash {
            record_wasm_store_metric(
                WasmStoreMetricOperation::ChunkUpload,
                WasmStoreMetricSource::TargetStore,
                WasmStoreMetricOutcome::Failed,
                WasmStoreMetricReason::HashMismatch,
            );
            record_wasm_store_publish_failed(WasmStoreMetricReason::HashMismatch);
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "template '{}' chunk {} hash mismatch for {}",
                    manifest.template_id, chunk_index, target_store.pid
                ),
            ));
        }

        record_wasm_store_metric(
            WasmStoreMetricOperation::ChunkUpload,
            WasmStoreMetricSource::TargetStore,
            WasmStoreMetricOutcome::Completed,
            WasmStoreMetricReason::Ok,
        );
        target_store
            .stored_chunk_hashes
            .as_mut()
            .expect("stored chunk hashes must be initialized")
            .insert(expected_hash);
        Ok(())
    }

    // Promote the manifest into the target store and mirror the approved root state.
    async fn promote_manifest_to_store_with_metrics(
        target_store: &PublicationStoreSnapshot,
        manifest: TemplateManifestResponse,
    ) -> Result<(), InternalError> {
        record_wasm_store_metric(
            WasmStoreMetricOperation::ManifestPromote,
            WasmStoreMetricSource::TargetStore,
            WasmStoreMetricOutcome::Started,
            WasmStoreMetricReason::Ok,
        );

        let input = TemplateManifestInput {
            template_id: manifest.template_id,
            role: manifest.role,
            version: manifest.version,
            payload_hash: manifest.payload_hash,
            payload_size_bytes: manifest.payload_size_bytes,
            store_binding: manifest.store_binding,
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(IcOps::now_secs()),
            created_at: manifest.created_at,
        };

        if let Err(err) = Self::promote_manifest_to_target_store(
            target_store.pid,
            target_store.binding.clone(),
            input,
        )
        .await
        {
            let reason = WasmStoreMetricReason::from_publication_error(&err);
            record_wasm_store_metric(
                WasmStoreMetricOperation::ManifestPromote,
                WasmStoreMetricSource::TargetStore,
                WasmStoreMetricOutcome::Failed,
                reason,
            );
            record_wasm_store_publish_failed(reason);
            return Err(err);
        }

        record_wasm_store_metric(
            WasmStoreMetricOperation::ManifestPromote,
            WasmStoreMetricSource::TargetStore,
            WasmStoreMetricOutcome::Completed,
            WasmStoreMetricReason::Ok,
        );
        canic_core::perf!("publish_promote_manifest");
        Ok(())
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
                let action_label = if placement.action == PublicationPlacementAction::Create {
                    "create"
                } else {
                    "publish"
                };
                let store_index = fleet
                    .store_index_for_binding(&placement.binding)
                    .ok_or_else(|| {
                        InternalError::workflow(
                            InternalErrorOrigin::Workflow,
                            format!("ws '{}' missing from fleet snapshot", placement.binding),
                        )
                    })?;

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
                    }
                    Err(err) if Self::is_store_capacity_exceeded(&err) => {
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
                            .ok_or_else(|| {
                                InternalError::workflow(
                                    InternalErrorOrigin::Workflow,
                                    format!("ws '{}' missing from fleet snapshot", retry.binding),
                                )
                            })?;
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
                        return Ok(());
                    }
                    Err(err) => return Err(err),
                }
            }
        }

        fleet.record_placement(&placement.binding, &manifest);
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

    // Reconcile root-owned approved manifest bindings against exact releases present in the fleet.
    pub async fn import_current_store_catalog() -> Result<(), InternalError> {
        let fleet = Self::snapshot_publication_store_fleet().await?;
        for manifest in Self::managed_release_manifests()? {
            let binding = Self::reconciled_binding_for_manifest(&fleet, &manifest)?;
            TemplateManifestOps::replace_approved_from_input(TemplateManifestInput {
                template_id: manifest.template_id,
                role: manifest.role,
                version: manifest.version,
                payload_hash: manifest.payload_hash,
                payload_size_bytes: manifest.payload_size_bytes,
                store_binding: binding,
                chunking_mode: manifest.chunking_mode,
                manifest_state: manifest.manifest_state,
                approved_at: manifest.approved_at,
                created_at: manifest.created_at,
            });
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

    // Stage one approved manifest into the target store and mirror it into root-owned state.
    async fn promote_manifest_to_target_store(
        target_store_pid: Principal,
        target_store_binding: WasmStoreBinding,
        manifest: TemplateManifestInput,
    ) -> Result<(), InternalError> {
        store_stage_manifest(
            target_store_pid,
            TemplateManifestInput {
                store_binding: target_store_binding.clone(),
                ..manifest.clone()
            },
        )
        .await?;

        TemplateManifestOps::replace_approved_from_input(TemplateManifestInput {
            store_binding: target_store_binding,
            ..manifest
        });

        Ok(())
    }
}

// Record one wasm-store metric point through the core API facade.
fn record_wasm_store_metric(
    operation: WasmStoreMetricOperation,
    source: WasmStoreMetricSource,
    outcome: WasmStoreMetricOutcome,
    reason: WasmStoreMetricReason,
) {
    WasmStoreMetricsApi::record(operation, source, outcome, reason);
}

// Record one target-store release publish failure reason.
fn record_wasm_store_publish_failed(reason: WasmStoreMetricReason) {
    record_wasm_store_metric(
        WasmStoreMetricOperation::ReleasePublish,
        WasmStoreMetricSource::TargetStore,
        WasmStoreMetricOutcome::Failed,
        reason,
    );
}

// Map publication failures into stable wasm-store metric reasons.
trait WasmStorePublicationError {
    fn from_publication_error(err: &InternalError) -> Self;
}

impl WasmStorePublicationError for WasmStoreMetricReason {
    fn from_publication_error(err: &InternalError) -> Self {
        if WasmStorePublicationWorkflow::is_store_capacity_exceeded(err) {
            Self::Capacity
        } else if err.public_error().is_some() {
            Self::StoreCall
        } else if err.to_string().contains("chunk") {
            Self::MissingChunk
        } else {
            Self::InvalidState
        }
    }
}
