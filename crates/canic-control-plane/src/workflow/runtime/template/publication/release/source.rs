use crate::{
    dto::template::{TemplateChunkSetInfoResponse, TemplateManifestResponse},
    ops::storage::template::TemplateChunkedOps,
    workflow::runtime::template::publication::{
        WasmStorePublicationWorkflow,
        store::{local_chunk, store_chunk, store_chunk_set_info},
    },
};
use canic_core::__control_plane_core as cp_core;
use canic_core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
};
use cp_core::{InternalError, cdk::types::Principal};

use super::super::super::{WASM_STORE_BOOTSTRAP_BINDING, store_pid_for_binding};
use super::metrics::{
    WasmStorePublicationError, record_wasm_store_metric, record_wasm_store_publish_failed,
};

impl WasmStorePublicationWorkflow {
    // Resolve the source store pid for one manifest-backed release, if it is store-backed.
    pub(super) fn source_store_pid_for_manifest(
        manifest: &TemplateManifestResponse,
    ) -> Result<Option<Principal>, InternalError> {
        if manifest.store_binding == WASM_STORE_BOOTSTRAP_BINDING {
            Ok(None)
        } else {
            store_pid_for_binding(&manifest.store_binding).map(Some)
        }
    }

    // Resolve deterministic chunk-set metadata for one manifest from its authoritative source.
    pub(super) async fn source_chunk_set_info_for_manifest(
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
    pub(super) async fn source_chunk_for_manifest(
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

    // Resolve source chunk hashes and record release-level failure if lookup fails.
    pub(super) async fn release_chunk_hashes(
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

    // Resolve one source chunk and record publication failure metrics when lookup fails.
    pub(super) async fn source_chunk_for_manifest_with_metrics(
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
}
