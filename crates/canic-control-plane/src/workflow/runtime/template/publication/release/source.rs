use crate::{
    dto::template::{TemplateChunkSetInfoResponse, TemplateManifestResponse},
    workflow::runtime::template::{
        publication::{
            WasmStorePublicationWorkflow,
            store::{store_chunk, store_chunk_set_info},
        },
        record_wasm_store_metric,
    },
};
use canic_core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
};
use canic_core::control_plane_support::{error::InternalError, ops::cost_guard::CostGuardPermit};

use super::metrics::{WasmStorePublicationError, record_wasm_store_publish_failed};
use crate::workflow::runtime::template::store_pid_for_binding;

impl WasmStorePublicationWorkflow {
    // Resolve deterministic chunk-set metadata for one manifest from its authoritative source.
    async fn source_chunk_set_info_for_manifest(
        publication_permit: &CostGuardPermit,
        manifest: &TemplateManifestResponse,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        let store_pid = store_pid_for_binding(&manifest.store_binding)?;
        store_chunk_set_info(
            publication_permit,
            store_pid,
            &manifest.template_id,
            &manifest.version,
        )
        .await
    }

    // Resolve one deterministic chunk for one manifest from its authoritative source.
    async fn source_chunk_for_manifest(
        publication_permit: &CostGuardPermit,
        manifest: &TemplateManifestResponse,
        chunk_index: u32,
    ) -> Result<Vec<u8>, InternalError> {
        let store_pid = store_pid_for_binding(&manifest.store_binding)?;
        store_chunk(
            publication_permit,
            store_pid,
            &manifest.template_id,
            &manifest.version,
            chunk_index,
        )
        .await
    }

    // Resolve source chunk hashes and record release-level failure if lookup fails.
    pub(super) async fn release_chunk_hashes(
        publication_permit: &CostGuardPermit,
        manifest: &TemplateManifestResponse,
    ) -> Result<Vec<Vec<u8>>, InternalError> {
        match Self::source_chunk_set_info_for_manifest(publication_permit, manifest).await {
            Ok(info) => Ok(info.chunk_hashes),
            Err(err) => {
                record_wasm_store_publish_failed(WasmStoreMetricReason::from_publication_error(
                    &err,
                ));
                Err(err)
            }
        }
    }

    // Resolve one source chunk and record publication failure metrics when lookup fails.
    pub(super) async fn source_chunk_for_manifest_with_metrics(
        publication_permit: &CostGuardPermit,
        manifest: &TemplateManifestResponse,
        chunk_index: u32,
    ) -> Result<Vec<u8>, InternalError> {
        match Self::source_chunk_for_manifest(publication_permit, manifest, chunk_index).await {
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
}
