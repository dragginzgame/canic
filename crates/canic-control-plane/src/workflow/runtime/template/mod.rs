pub mod publication;

pub use publication::WasmStorePublicationWorkflow;

use crate::{
    dto::template::{TemplateChunkSetInfoResponse, TemplateManifestResponse},
    ids::{TemplateId, TemplateVersion, WasmStoreBinding},
    ops::storage::{
        state::subnet::SubnetStateOps,
        template::{TemplateChunkedOps, TemplateManifestOps},
    },
};
use candid::utils::ArgumentEncoder;
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
        crate::ids::TemplateChunkingMode::Inline => Err(InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!(
                "inline module sources are no longer supported; role '{}' source '{}' must be staged and published through a wasm_store",
                manifest.role, manifest.template_id
            ),
        )),
        crate::ids::TemplateChunkingMode::Chunked => {
            if manifest.store_binding == WASM_STORE_BOOTSTRAP_BINDING {
                let (store_pid, info) = resolved_bootstrap_chunk_set_for_manifest(manifest).await?;

                return Ok(ApprovedModuleSource {
                    source_canister: store_pid,
                    source_label: manifest.template_id.as_str().to_string(),
                    module_hash: manifest.payload_hash.clone(),
                    chunk_hashes: info.chunk_hashes,
                    payload_size_bytes: manifest.payload_size_bytes,
                });
            }

            let (store_pid, info) = resolved_store_chunk_set_for_manifest(manifest).await?;

            Ok(ApprovedModuleSource {
                source_canister: store_pid,
                source_label: manifest.template_id.as_str().to_string(),
                module_hash: manifest.payload_hash.clone(),
                chunk_hashes: info.chunk_hashes,
                payload_size_bytes: manifest.payload_size_bytes,
            })
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

    ensure_store_chunk_hashes_present(store_pid, &manifest.template_id, &manifest.version, &info)
        .await?;

    Ok((store_pid, info))
}

// Upload any missing root-local staged chunks into the current canister's
// management chunk store before install uses it as the bootstrap source.
async fn ensure_bootstrap_chunk_hashes_present(
    template_id: &TemplateId,
    version: &TemplateVersion,
    info: &TemplateChunkSetInfoResponse,
) -> Result<(), InternalError> {
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
        return Ok(());
    }

    for (chunk_index, expected_hash) in info.chunk_hashes.iter().cloned().enumerate() {
        if stored_hashes.contains(&expected_hash) {
            continue;
        }

        let chunk_index = u32::try_from(chunk_index).map_err(|_| {
            InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!("template '{template_id}' exceeds supported chunk indexing bounds"),
            )
        })?;
        let bytes = TemplateChunkedOps::chunk_response(template_id, version, chunk_index)?.bytes;
        let uploaded_hash = MgmtOps::upload_chunk(store_pid, bytes).await?;

        if uploaded_hash != expected_hash {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "template '{template_id}' bootstrap chunk {chunk_index} uploaded hash mismatch for root {store_pid}",
                ),
            ));
        }
    }

    Ok(())
}

// Upload any missing deterministic chunks into the selected store's local
// management chunk store before install uses it as the source canister.
async fn ensure_store_chunk_hashes_present(
    store_pid: Principal,
    template_id: &TemplateId,
    version: &TemplateVersion,
    info: &TemplateChunkSetInfoResponse,
) -> Result<(), InternalError> {
    let stored_hashes = MgmtOps::stored_chunks(store_pid)
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>();

    if info
        .chunk_hashes
        .iter()
        .all(|expected_hash| stored_hashes.contains(expected_hash))
    {
        return Ok(());
    }

    let chunks =
        publication::store_chunks(store_pid, template_id, version, info.chunk_hashes.len()).await?;

    for (chunk_index, (expected_hash, bytes)) in info
        .chunk_hashes
        .iter()
        .cloned()
        .zip(chunks.into_iter())
        .enumerate()
    {
        if stored_hashes.contains(&expected_hash) {
            continue;
        }

        let uploaded_hash = MgmtOps::upload_chunk(store_pid, bytes).await?;
        if uploaded_hash != expected_hash {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "template '{template_id}' chunk {chunk_index} uploaded hash mismatch for store {store_pid}",
                ),
            ));
        }
    }

    Ok(())
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
