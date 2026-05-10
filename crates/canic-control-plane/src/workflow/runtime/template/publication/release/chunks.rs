use crate::{
    dto::template::TemplateManifestResponse,
    workflow::runtime::template::publication::{
        WasmStorePublicationWorkflow, fleet::PublicationStoreSnapshot, store::TemplateChunkInputRef,
    },
};
use canic_core::__control_plane_core as cp_core;
use canic_core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
};
use cp_core::{InternalError, InternalErrorOrigin, cdk::types::Principal};

use super::metrics::{
    WasmStorePublicationError, record_wasm_store_metric, record_wasm_store_publish_failed,
};

impl WasmStorePublicationWorkflow {
    // Publish every source chunk to the target store and refresh install-cache chunks.
    pub(super) async fn publish_manifest_chunks_to_store(
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

        if let Err(err) = super::super::super::call_store_result::<(), _>(
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
}
