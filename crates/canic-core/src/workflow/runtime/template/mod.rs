pub mod install;
pub mod publication;

pub use install::TemplateInstallWorkflow;
pub use publication::WasmStorePublicationWorkflow;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::{Principal, WasmModule},
    dto::{
        error::Error,
        template::{
            TemplateChunkResponse, TemplateChunkSetInfoResponse, TemplateManifestResponse,
            WasmStoreCatalogEntryResponse, WasmStoreStatusResponse,
        },
    },
    ids::{CanisterRole, TemplateId, TemplateVersion, WasmStoreBinding},
    ops::{
        ic::{IcOps, call::CallOps, mgmt::MgmtOps},
        runtime::template::EmbeddedTemplatePayloadOps,
        storage::{state::subnet::SubnetStateOps, template::TemplateManifestOps},
    },
    protocol,
};
use candid::utils::ArgumentEncoder;
use std::collections::BTreeSet;

// Maximum management-canister chunk-store payload accepted per call. Use the
// full 1 MiB limit to minimize round-trips without exceeding install bounds.
const DEFAULT_WASM_STORE_PUBLISH_CHUNK_BYTES: usize = 1024 * 1024;
const WASM_STORE_ROLE: CanisterRole = CanisterRole::WASM_STORE;
const WASM_STORE_BOOTSTRAP_BINDING: WasmStoreBinding = WasmStoreBinding::new("bootstrap");

// Build a deterministic template identifier for the embedded release set.
fn embedded_template_id(role: &CanisterRole) -> TemplateId {
    TemplateId::owned(format!("embedded:{}", role.as_str()))
}

// Resolve the currently embedded payload for an approved manifest and verify it matches.
fn verified_embedded_wasm_for_manifest(
    manifest: &TemplateManifestResponse,
) -> Result<WasmModule, InternalError> {
    let wasm = EmbeddedTemplatePayloadOps::try_get(&manifest.template_id)?;
    let module_hash = wasm.module_hash();

    if module_hash != manifest.payload_hash {
        return Err(InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!(
                "approved template '{}' hash mismatch for role '{}'",
                manifest.template_id, manifest.role
            ),
        ));
    }

    if wasm.len() as u64 != manifest.payload_size_bytes {
        return Err(InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!(
                "approved template '{}' size mismatch for role '{}'",
                manifest.template_id, manifest.role
            ),
        ));
    }

    Ok(wasm)
}

// Fetch the approved embedded catalog from one wasm store.
async fn store_catalog(
    store_pid: Principal,
) -> Result<Vec<WasmStoreCatalogEntryResponse>, InternalError> {
    call_store_result(store_pid, protocol::CANIC_WASM_STORE_CATALOG, ()).await
}

// Fetch deterministic chunk-set metadata for one release from one wasm store.
async fn store_chunk_set_info(
    store_pid: Principal,
    template_id: &TemplateId,
    version: &TemplateVersion,
) -> Result<TemplateChunkSetInfoResponse, InternalError> {
    call_store_result(
        store_pid,
        protocol::CANIC_WASM_STORE_INFO,
        (
            template_id.as_str().to_string(),
            version.as_str().to_string(),
        ),
    )
    .await
}

// Fetch current occupied-byte and retention state from one wasm store.
async fn store_status(store_pid: Principal) -> Result<WasmStoreStatusResponse, InternalError> {
    call_store_result(store_pid, protocol::CANIC_WASM_STORE_STATUS, ()).await
}

// Mark one local wasm store as prepared for store-local GC execution.
async fn store_prepare_gc(store_pid: Principal) -> Result<(), InternalError> {
    call_store_result(store_pid, protocol::CANIC_WASM_STORE_PREPARE_GC, ()).await
}

// Mark one local wasm store as actively executing store-local GC.
async fn store_begin_gc(store_pid: Principal) -> Result<(), InternalError> {
    call_store_result(store_pid, protocol::CANIC_WASM_STORE_BEGIN_GC, ()).await
}

// Mark one local wasm store as having completed the current local GC pass.
async fn store_complete_gc(store_pid: Principal) -> Result<(), InternalError> {
    call_store_result(store_pid, protocol::CANIC_WASM_STORE_COMPLETE_GC, ()).await
}

// Fetch all deterministic chunks for one release from one wasm store.
async fn store_chunks(
    store_pid: Principal,
    template_id: &TemplateId,
    version: &TemplateVersion,
    chunk_count: usize,
) -> Result<Vec<Vec<u8>>, InternalError> {
    let mut chunks = Vec::with_capacity(chunk_count);

    for chunk_index in 0..chunk_count {
        let chunk_index = u32::try_from(chunk_index).map_err(|_| {
            InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!("template '{template_id}' exceeds supported chunk indexing bounds"),
            )
        })?;
        let response: TemplateChunkResponse = call_store_result(
            store_pid,
            protocol::CANIC_WASM_STORE_CHUNK,
            (
                template_id.as_str().to_string(),
                version.as_str().to_string(),
                chunk_index,
            ),
        )
        .await?;
        chunks.push(response.bytes);
    }

    Ok(chunks)
}

// Resolve deterministic chunk metadata for one manifest-bound store release and verify it is installable.
async fn resolved_store_chunk_set_for_manifest(
    manifest: &TemplateManifestResponse,
) -> Result<(Principal, TemplateChunkSetInfoResponse), InternalError> {
    if manifest.store_binding == WASM_STORE_BOOTSTRAP_BINDING {
        let store_pid = IcOps::canister_self();
        let info =
            TemplateManifestOps::chunk_set_info_response(&manifest.template_id, &manifest.version)?;

        if info.chunk_hashes.is_empty() {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "template '{}' chunk metadata is incomplete for local bootstrap store",
                    manifest.template_id
                ),
            ));
        }

        ensure_local_chunk_hashes_present(&manifest.template_id, &manifest.version, &info).await?;
        return Ok((store_pid, info));
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

// Return deterministic chunk bytes from the current canister's local bootstrap source.
fn local_chunks(
    template_id: &TemplateId,
    version: &TemplateVersion,
    chunk_count: usize,
) -> Result<Vec<Vec<u8>>, InternalError> {
    let mut chunks = Vec::with_capacity(chunk_count);

    for chunk_index in 0..chunk_count {
        let chunk_index = u32::try_from(chunk_index).map_err(|_| {
            InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!("template '{template_id}' exceeds supported chunk indexing bounds"),
            )
        })?;
        let response = TemplateManifestOps::chunk_response(template_id, version, chunk_index)?;
        chunks.push(response.bytes);
    }

    Ok(chunks)
}

// Upload any missing deterministic chunks into the current canister's local
// management chunk store before bootstrap installs use it as the source canister.
async fn ensure_local_chunk_hashes_present(
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

    let chunks = local_chunks(template_id, version, info.chunk_hashes.len())?;

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
                    "template '{template_id}' chunk {chunk_index} uploaded hash mismatch for local bootstrap store"
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

    let chunks = store_chunks(store_pid, template_id, version, info.chunk_hashes.len()).await?;

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

// Resolve the configured logical binding for one registered store canister id.
fn store_binding_for_pid(store_pid: Principal) -> Result<WasmStoreBinding, InternalError> {
    SubnetStateOps::wasm_store_binding_for_pid(store_pid).ok_or_else(|| {
        InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("wasm store {store_pid} is not registered"),
        )
    })
}

// Split a wasm module into deterministic fixed-size chunks for store publication.
fn split_chunks(bytes: &[u8], chunk_bytes: usize) -> Vec<Vec<u8>> {
    bytes.chunks(chunk_bytes).map(<[u8]>::to_vec).collect()
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
