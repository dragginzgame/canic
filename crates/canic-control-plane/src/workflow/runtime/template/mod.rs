pub mod publication;

pub use publication::WasmStorePublicationWorkflow;

use crate::{
    dto::template::{TemplateChunkSetInfoResponse, TemplateManifestResponse},
    ids::{TemplateId, TemplateReleaseKey, TemplateVersion, WasmStoreBinding},
    ops::storage::{
        state::subnet::SubnetStateOps,
        template::{TemplateChunkedOps, TemplateManifestOps},
    },
};
use candid::utils::ArgumentEncoder;
use canic_core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
    WasmStoreMetricsApi,
};
use canic_core::api::runtime::install::ApprovedModuleSource;
use canic_core::{__control_plane_core as cp_core, dto::error::Error};
use cp_core::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    ops::ic::{IcOps, call::CallOps, mgmt::MgmtOps},
    protocol,
};
use std::collections::BTreeSet;

const WASM_STORE_BOOTSTRAP_BINDING: WasmStoreBinding = WasmStoreBinding::new("bootstrap");

// Build one stable release label for logs and install-source reporting.
fn release_source_label(template_id: &TemplateId, version: &TemplateVersion) -> String {
    TemplateReleaseKey::new(template_id.clone(), version.clone()).to_string()
}

// Resolve the approved chunk-backed module source for one role through the current store binding.
pub async fn resolved_approved_module_source_for_role(
    role: &crate::ids::CanisterRole,
) -> Result<ApprovedModuleSource, InternalError> {
    let manifest = TemplateManifestOps::approved_for_role_response(role)?;
    approved_module_source_from_manifest(&manifest).await
}

// Convert one approved manifest into the neutral chunk-backed install source contract.
pub async fn approved_module_source_from_manifest(
    manifest: &TemplateManifestResponse,
) -> Result<ApprovedModuleSource, InternalError> {
    match manifest.chunking_mode {
        crate::ids::TemplateChunkingMode::Inline => {
            record_wasm_store_metric(
                WasmStoreMetricOperation::SourceResolve,
                WasmStoreMetricSource::Store,
                WasmStoreMetricOutcome::Failed,
                WasmStoreMetricReason::UnsupportedInline,
            );
            Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "inline module sources are no longer supported; role '{}' source '{}' must be staged and published through a wasm_store",
                    manifest.role, manifest.template_id
                ),
            ))
        }
        crate::ids::TemplateChunkingMode::Chunked => {
            if manifest.store_binding == WASM_STORE_BOOTSTRAP_BINDING {
                record_wasm_store_metric(
                    WasmStoreMetricOperation::SourceResolve,
                    WasmStoreMetricSource::Bootstrap,
                    WasmStoreMetricOutcome::Started,
                    WasmStoreMetricReason::Ok,
                );
                let (store_pid, info) =
                    match resolved_bootstrap_chunk_set_for_manifest(manifest).await {
                        Ok(source) => source,
                        Err(err) => {
                            record_wasm_store_metric(
                                WasmStoreMetricOperation::SourceResolve,
                                WasmStoreMetricSource::Bootstrap,
                                WasmStoreMetricOutcome::Failed,
                                WasmStoreMetricReason::from_manifest_source_error(&err),
                            );
                            return Err(err);
                        }
                    };

                record_wasm_store_metric(
                    WasmStoreMetricOperation::SourceResolve,
                    WasmStoreMetricSource::Bootstrap,
                    WasmStoreMetricOutcome::Completed,
                    WasmStoreMetricReason::Ok,
                );

                return Ok(ApprovedModuleSource::chunked(
                    store_pid,
                    release_source_label(&manifest.template_id, &manifest.version),
                    manifest.payload_hash.clone(),
                    info.chunk_hashes,
                    manifest.payload_size_bytes,
                ));
            }

            record_wasm_store_metric(
                WasmStoreMetricOperation::SourceResolve,
                WasmStoreMetricSource::Store,
                WasmStoreMetricOutcome::Started,
                WasmStoreMetricReason::Ok,
            );
            let (store_pid, info) = match resolved_store_chunk_set_for_manifest(manifest).await {
                Ok(source) => source,
                Err(err) => {
                    record_wasm_store_metric(
                        WasmStoreMetricOperation::SourceResolve,
                        WasmStoreMetricSource::Store,
                        WasmStoreMetricOutcome::Failed,
                        WasmStoreMetricReason::from_manifest_source_error(&err),
                    );
                    return Err(err);
                }
            };

            record_wasm_store_metric(
                WasmStoreMetricOperation::SourceResolve,
                WasmStoreMetricSource::Store,
                WasmStoreMetricOutcome::Completed,
                WasmStoreMetricReason::Ok,
            );

            Ok(ApprovedModuleSource::chunked(
                store_pid,
                release_source_label(&manifest.template_id, &manifest.version),
                manifest.payload_hash.clone(),
                info.chunk_hashes,
                manifest.payload_size_bytes,
            ))
        }
    }
}

// Resolve the root-local bootstrap chunk source for one manifest and make sure
// the current canister's management chunk store contains the expected payload.
async fn resolved_bootstrap_chunk_set_for_manifest(
    manifest: &TemplateManifestResponse,
) -> Result<(Principal, TemplateChunkSetInfoResponse), InternalError> {
    let store_pid = IcOps::canister_self();
    let info =
        TemplateChunkedOps::chunk_set_info_response(&manifest.template_id, &manifest.version)?;

    if info.chunk_hashes.is_empty() {
        return Err(InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!(
                "template '{}' bootstrap chunk metadata is incomplete",
                manifest.template_id
            ),
        ));
    }

    ensure_bootstrap_chunk_hashes_present(&manifest.template_id, &manifest.version, &info).await?;

    Ok((store_pid, info))
}

// Resolve deterministic chunk metadata for one manifest-bound store release and verify it is installable.
async fn resolved_store_chunk_set_for_manifest(
    manifest: &TemplateManifestResponse,
) -> Result<(Principal, TemplateChunkSetInfoResponse), InternalError> {
    if manifest.store_binding == WASM_STORE_BOOTSTRAP_BINDING {
        return Err(InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!(
                "template '{}' uses the local bootstrap store, which is only installable through the root control-plane path",
                manifest.template_id
            ),
        ));
    }

    let store_pid = store_pid_for_binding(&manifest.store_binding)?;
    let info: TemplateChunkSetInfoResponse = call_store_result(
        store_pid,
        protocol::CANIC_WASM_STORE_INFO,
        (
            manifest.template_id.as_str().to_string(),
            manifest.version.as_str().to_string(),
        ),
    )
    .await?;

    if info.chunk_hashes.is_empty() {
        return Err(InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!(
                "template '{}' chunk metadata is incomplete for store {}",
                manifest.template_id, store_pid
            ),
        ));
    }

    Ok((store_pid, info))
}

// Upload any missing root-local staged chunks into the current canister's
// management chunk store before install uses it as the bootstrap source.
async fn ensure_bootstrap_chunk_hashes_present(
    template_id: &TemplateId,
    version: &TemplateVersion,
    info: &TemplateChunkSetInfoResponse,
) -> Result<(), InternalError> {
    record_wasm_store_metric(
        WasmStoreMetricOperation::BootstrapChunkSync,
        WasmStoreMetricSource::Bootstrap,
        WasmStoreMetricOutcome::Started,
        WasmStoreMetricReason::Ok,
    );
    let store_pid = IcOps::canister_self();
    let stored_hashes = MgmtOps::stored_chunks(store_pid)
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>();

    if info
        .chunk_hashes
        .iter()
        .all(|expected_hash| stored_hashes.contains(expected_hash))
    {
        record_wasm_store_metric(
            WasmStoreMetricOperation::BootstrapChunkSync,
            WasmStoreMetricSource::Bootstrap,
            WasmStoreMetricOutcome::Completed,
            WasmStoreMetricReason::CacheHit,
        );
        return Ok(());
    }

    for (chunk_index, expected_hash) in info.chunk_hashes.iter().cloned().enumerate() {
        if stored_hashes.contains(&expected_hash) {
            record_wasm_store_metric(
                WasmStoreMetricOperation::ChunkUpload,
                WasmStoreMetricSource::Bootstrap,
                WasmStoreMetricOutcome::Skipped,
                WasmStoreMetricReason::CacheHit,
            );
            continue;
        }

        record_wasm_store_metric(
            WasmStoreMetricOperation::ChunkUpload,
            WasmStoreMetricSource::Bootstrap,
            WasmStoreMetricOutcome::Started,
            WasmStoreMetricReason::CacheMiss,
        );
        let chunk_index = u32::try_from(chunk_index).map_err(|_| {
            InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!("template '{template_id}' exceeds supported chunk indexing bounds"),
            )
        })?;
        let bytes = TemplateChunkedOps::chunk_response(template_id, version, chunk_index)?.bytes;
        let uploaded_hash = match MgmtOps::upload_chunk(store_pid, bytes).await {
            Ok(uploaded_hash) => uploaded_hash,
            Err(err) => {
                record_wasm_store_metric(
                    WasmStoreMetricOperation::ChunkUpload,
                    WasmStoreMetricSource::Bootstrap,
                    WasmStoreMetricOutcome::Failed,
                    WasmStoreMetricReason::ManagementCall,
                );
                return Err(err);
            }
        };

        if uploaded_hash != expected_hash {
            record_wasm_store_metric(
                WasmStoreMetricOperation::ChunkUpload,
                WasmStoreMetricSource::Bootstrap,
                WasmStoreMetricOutcome::Failed,
                WasmStoreMetricReason::HashMismatch,
            );
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "template '{template_id}' bootstrap chunk {chunk_index} uploaded hash mismatch for root {store_pid}",
                ),
            ));
        }

        record_wasm_store_metric(
            WasmStoreMetricOperation::ChunkUpload,
            WasmStoreMetricSource::Bootstrap,
            WasmStoreMetricOutcome::Completed,
            WasmStoreMetricReason::Ok,
        );
    }

    record_wasm_store_metric(
        WasmStoreMetricOperation::BootstrapChunkSync,
        WasmStoreMetricSource::Bootstrap,
        WasmStoreMetricOutcome::Completed,
        WasmStoreMetricReason::CacheMiss,
    );

    Ok(())
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

// Map install-source resolution failures into stable wasm-store metric reasons.
trait WasmStoreManifestSourceError {
    fn from_manifest_source_error(err: &InternalError) -> Self;
}

impl WasmStoreManifestSourceError for WasmStoreMetricReason {
    fn from_manifest_source_error(err: &InternalError) -> Self {
        if err.to_string().contains("not registered") {
            Self::MissingManifest
        } else if err.to_string().contains("chunk") {
            Self::MissingChunk
        } else if err.public_error().is_some() {
            Self::StoreCall
        } else {
            Self::InvalidState
        }
    }
}

// Resolve the currently configured store canister id for one approved binding.
fn store_pid_for_binding(binding: &WasmStoreBinding) -> Result<Principal, InternalError> {
    SubnetStateOps::wasm_store_pid(binding).ok_or_else(|| {
        InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("wasm store binding '{binding}' is not registered"),
        )
    })
}

// Call one wasm-store endpoint that returns `Result<T, Error>`.
async fn call_store_result<T, A>(
    store_pid: Principal,
    method: &str,
    arg: A,
) -> Result<T, InternalError>
where
    T: candid::CandidType + serde::de::DeserializeOwned,
    A: ArgumentEncoder,
{
    let call = CallOps::unbounded_wait(store_pid, method)
        .with_args(arg)?
        .execute()
        .await?;
    let call_res: Result<T, Error> = call.candid::<Result<T, Error>>()?;

    call_res.map_err(InternalError::public)
}

#[cfg(test)]
mod tests {
    use super::release_source_label;
    use crate::ids::{TemplateId, TemplateVersion};

    #[test]
    fn release_source_label_includes_version() {
        let label = release_source_label(
            &TemplateId::new("embedded:user_hub"),
            &TemplateVersion::new("0.20.2"),
        );

        assert_eq!(label, "embedded:user_hub@0.20.2");
    }
}
