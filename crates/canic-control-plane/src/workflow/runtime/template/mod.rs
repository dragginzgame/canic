mod client;
pub mod publication;

pub(in crate::workflow) use client::WasmStoreInternalClient;
pub use publication::WasmStorePublicationWorkflow;

use crate::{
    ids::{TemplateId, TemplateReleaseKey, TemplateVersion, WasmStoreBinding},
    ops::storage::state::root_wasm_store::RootWasmStoreStateOps,
};
use canic_core::{
    api::{
        lifecycle::metrics::{
            WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason,
            WasmStoreMetricSource, WasmStoreMetricsApi,
        },
        runtime::install::ApprovedModuleSource,
    },
    cdk::{types::Principal, utils::hash::wasm_hash},
    control_plane_support::error::InternalError,
    diagnostics::codes,
};

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

// Record one wasm-store metric point through the core API facade.
fn record_wasm_store_metric(
    operation: WasmStoreMetricOperation,
    source: WasmStoreMetricSource,
    outcome: WasmStoreMetricOutcome,
    reason: WasmStoreMetricReason,
) {
    WasmStoreMetricsApi::record(operation, source, outcome, reason);
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
        ids::{TemplateId, TemplateVersion, WasmStoreBinding, WasmStoreGcMode},
        ops::storage::state::root_wasm_store::{
            PublicationStoreStateTestInput, RootWasmStoreStateOps, WasmStoreStateTestInput,
        },
    };
    use canic_core::{cdk::types::Principal, diagnostics::codes};

    fn import_store_inventory(wasm_stores: Vec<WasmStoreStateTestInput>) {
        RootWasmStoreStateOps::import_test_state(
            PublicationStoreStateTestInput {
                active_binding: None,
                detached_binding: None,
                retired_binding: None,
                generation: 0,
                changed_at: 0,
                retired_at: 0,
            },
            wasm_stores,
        );
    }

    #[test]
    fn manifest_source_requires_the_exact_registered_store_binding() {
        let binding = WasmStoreBinding::new("primary");
        let pid = Principal::from_slice(&[31; 29]);
        import_store_inventory(vec![WasmStoreStateTestInput {
            binding: binding.clone(),
            pid,
            created_at: 10,
            gc_mode: WasmStoreGcMode::Normal,
            gc_changed_at: 0,
            prepared_at: None,
            started_at: None,
            completed_at: None,
            runs_completed: 0,
        }]);
        assert_eq!(store_pid_for_binding(&binding).unwrap(), pid);
        let error = store_pid_for_binding(&WasmStoreBinding::new("unregistered")).unwrap_err();
        assert_eq!(
            error.public_error().code(),
            codes::WASM_STORE_MANIFEST_MISSING.raw_code()
        );
        import_store_inventory(Vec::new());
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
