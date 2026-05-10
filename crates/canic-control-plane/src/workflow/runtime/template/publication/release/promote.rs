use crate::{
    dto::template::{TemplateManifestInput, TemplateManifestResponse},
    ids::{TemplateChunkingMode, TemplateManifestState, WasmStoreBinding},
    ops::storage::template::TemplateManifestOps,
    workflow::runtime::template::publication::{
        WasmStorePublicationWorkflow, fleet::PublicationStoreSnapshot, store::store_stage_manifest,
    },
};
use canic_core::__control_plane_core as cp_core;
use canic_core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
};
use cp_core::{InternalError, cdk::types::Principal, ops::ic::IcOps};

use super::metrics::{
    WasmStorePublicationError, record_wasm_store_metric, record_wasm_store_publish_failed,
};

impl WasmStorePublicationWorkflow {
    // Promote the manifest into the target store and mirror the approved root state.
    pub(super) async fn promote_manifest_to_store_with_metrics(
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
