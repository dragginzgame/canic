mod client;
pub mod publication;

pub(in crate::workflow) use client::WasmStoreInternalClient;
pub use publication::WasmStorePublicationWorkflow;

use crate::{
    dto::template::{TemplateChunkSetInfoResponse, TemplateManifestResponse},
    ids::{TemplateId, TemplateReleaseKey, TemplateVersion, WasmStoreBinding},
    ops::storage::{state::root_wasm_store::RootWasmStoreStateOps, template::TemplateManifestOps},
};
use canic_core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
    WasmStoreMetricsApi,
};
use canic_core::api::runtime::install::ApprovedModuleSource;
use canic_core::cdk::types::Principal;
use canic_core::cdk::utils::hash::wasm_hash;
use canic_core::control_plane_support::error::InternalError;
use canic_core::diagnostics::codes;

/// Read and verify one complete chunked payload from an exact Store.
pub(in crate::workflow) async fn exact_store_payload_bytes(
    store_pid: Principal,
    template_id: &TemplateId,
    version: &TemplateVersion,
    expected_payload_hash: &[u8],
    expected_payload_size_bytes: u64,
) -> Result<Vec<u8>, InternalError> {
    let client = WasmStoreInternalClient::new(store_pid);
    let info = client.info(template_id, version).await?;
    if info.chunk_hashes.is_empty() {
        return Err(InternalError::invalid_input());
    }
    let capacity =
        usize::try_from(expected_payload_size_bytes).map_err(|_| InternalError::invalid_input())?;
    let mut payload = Vec::with_capacity(capacity);
    for (index, expected_chunk_hash) in info.chunk_hashes.iter().enumerate() {
        let chunk_index = u32::try_from(index).map_err(|_| InternalError::invalid_input())?;
        let chunk = client.chunk(template_id, version, chunk_index).await?;
        if &wasm_hash(&chunk) != expected_chunk_hash {
            return Err(InternalError::conflict());
        }
        payload.extend_from_slice(&chunk);
    }
    if payload.len() as u64 != expected_payload_size_bytes
        || wasm_hash(&payload) != expected_payload_hash
    {
        return Err(InternalError::conflict());
    }
    Ok(payload)
}

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

/// Resolve one exact root-local Store artifact selected by protected release-set evidence.
pub(in crate::workflow) async fn resolved_root_store_module_source(
    store_pid: Principal,
    release_build_id: canic_core::ids::ReleaseBuildId,
    role: &crate::ids::CanisterRole,
    payload_hash: [u8; 32],
    payload_size_bytes: u64,
) -> Result<ApprovedModuleSource, InternalError> {
    let template_id = TemplateId::owned(format!(
        "{}{role}",
        canic_core::dto::root_store::ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX
    ));
    let version = TemplateVersion::owned(release_build_id.to_string());
    let info = WasmStoreInternalClient::new(store_pid)
        .info(&template_id, &version)
        .await?;
    if info.chunk_hashes.is_empty()
        || info
            .chunk_hashes
            .iter()
            .any(|chunk_hash| chunk_hash.len() != 32)
    {
        return Err(InternalError::lifecycle_failure());
    }

    Ok(ApprovedModuleSource::chunked(
        store_pid,
        release_source_label(&template_id, &version),
        payload_hash.to_vec(),
        info.chunk_hashes,
        payload_size_bytes,
    ))
}

// Convert one approved manifest into the neutral chunk-backed install source contract.
async fn approved_module_source_from_manifest(
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
            Err(InternalError::lifecycle_failure())
        }
        crate::ids::TemplateChunkingMode::Chunked => {
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

// Resolve deterministic chunk metadata for one manifest-bound store release and verify it is installable.
async fn resolved_store_chunk_set_for_manifest(
    manifest: &TemplateManifestResponse,
) -> Result<(Principal, TemplateChunkSetInfoResponse), InternalError> {
    let store_pid = store_pid_for_binding(&manifest.store_binding)?;
    let info = WasmStoreInternalClient::new(store_pid)
        .info(&manifest.template_id, &manifest.version)
        .await?;

    if info.chunk_hashes.is_empty() {
        return Err(InternalError::lifecycle_failure());
    }

    Ok((store_pid, info))
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
        match err.public_error().code() {
            code if code == codes::WASM_STORE_CHUNK_MISSING.raw_code() => Self::MissingChunk,
            code if code == codes::DIGEST_CONFLICT.raw_code() => Self::HashMismatch,
            code if code == codes::WASM_STORE_MANIFEST_MISSING.raw_code() => Self::MissingManifest,
            _ => Self::StoreCall,
        }
    }
}

// Resolve the currently configured store canister id for one approved binding.
fn store_pid_for_binding(binding: &WasmStoreBinding) -> Result<Principal, InternalError> {
    RootWasmStoreStateOps::wasm_store_pid(binding)
        .ok_or_else(|| InternalError::public(codes::WASM_STORE_MANIFEST_MISSING))
}

#[cfg(test)]
mod tests {
    use super::{release_source_label, store_pid_for_binding};
    use crate::{
        ids::{TemplateId, TemplateVersion, WasmStoreBinding},
        storage::stable::state::root_wasm_store::{
            RootWasmStoreState, RootWasmStoreStateData, RootWasmStoreStateRecord,
            WasmStoreGcRecord, WasmStoreRecord,
        },
    };
    use canic_core::{cdk::types::Principal, diagnostics::codes};

    #[test]
    fn manifest_source_requires_the_exact_registered_store_binding() {
        let binding = WasmStoreBinding::new("primary");
        let pid = Principal::from_slice(&[31; 29]);
        RootWasmStoreState::import(RootWasmStoreStateData {
            record: RootWasmStoreStateRecord {
                wasm_stores: vec![WasmStoreRecord {
                    binding: binding.clone(),
                    pid,
                    created_at: 10,
                    gc: WasmStoreGcRecord::default(),
                }],
                ..RootWasmStoreStateRecord::default()
            },
        });
        assert_eq!(store_pid_for_binding(&binding).unwrap(), pid);
        let error = store_pid_for_binding(&WasmStoreBinding::new("unregistered")).unwrap_err();
        assert_eq!(
            error.public_error().code(),
            codes::WASM_STORE_MANIFEST_MISSING.raw_code()
        );
        RootWasmStoreState::import(RootWasmStoreStateData::default());
    }

    #[test]
    fn release_source_label_includes_version() {
        let label = release_source_label(
            &TemplateId::new("embedded:user_hub"),
            &TemplateVersion::new("0.20.2"),
        );

        assert_eq!(label, "embedded:user_hub@0.20.2");
    }
}
