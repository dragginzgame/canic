use crate::{
    dto::template::{
        TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput, TemplateManifestResponse,
    },
    workflow::runtime::template::{
        WasmStoreInternalClient,
        publication::{WasmStorePublicationWorkflow, fleet::PublicationStoreSnapshot},
    },
};
use canic_core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
};
use canic_core::cdk::types::Principal;
use canic_core::control_plane_support::{error::InternalError, ops::cost_guard::CostGuardPermit};
use canic_core::{log, log::Topic};

use super::metrics::{
    WasmStorePublicationError, record_wasm_store_metric, record_wasm_store_publish_failed,
};

impl WasmStorePublicationWorkflow {
    // Publish one approved manifest into the target store from its authoritative source.
    pub(super) async fn publish_manifest_to_store(
        target_store: &mut PublicationStoreSnapshot,
        manifest: TemplateManifestResponse,
        publication_permit: &CostGuardPermit,
    ) -> Result<(), InternalError> {
        record_wasm_store_metric(
            WasmStoreMetricOperation::ReleasePublish,
            WasmStoreMetricSource::TargetStore,
            WasmStoreMetricOutcome::Started,
            WasmStoreMetricReason::Ok,
        );
        let chunk_hashes = Self::release_chunk_hashes(publication_permit, &manifest).await?;

        target_store
            .ensure_stored_chunk_hashes(publication_permit)
            .await?;
        Self::prepare_target_store_for_manifest(
            publication_permit,
            target_store.pid,
            &manifest,
            &chunk_hashes,
        )
        .await?;
        Self::publish_manifest_chunks_to_store(
            publication_permit,
            target_store,
            &manifest,
            &chunk_hashes,
        )
        .await?;
        Self::promote_manifest_to_store_with_metrics(
            publication_permit,
            target_store,
            manifest.clone(),
        )
        .await?;

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

    // Prepare the target store for one manifest's canonical chunk set.
    async fn prepare_target_store_for_manifest(
        publication_permit: &CostGuardPermit,
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
            WasmStoreInternalClient::new(target_store_pid)
                .prepare_chunk_set(
                    publication_permit,
                    TemplateChunkSetPrepareInput {
                        template_id: manifest.template_id.clone(),
                        version: manifest.version.clone(),
                        payload_hash: manifest.payload_hash.clone(),
                        payload_size_bytes: manifest.payload_size_bytes,
                        chunk_hashes: chunk_hashes.to_vec(),
                    },
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
}
