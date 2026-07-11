use super::{
    artifact::read_release_artifact,
    call::icp_call_on_network,
    candid::{idl_blob, idl_text},
    progress::StageProgress,
};
use crate::icp::LocalReplicaTarget;
use canic_core::{CANIC_WASM_CHUNK_BYTES, cdk::utils::hash::decode_hex, protocol};
use std::{path::Path, time::Instant};

use super::super::ReleaseSetEntry;

// Stage one manifest, prepare its chunk set, and publish all chunk bytes into root.
#[expect(
    clippy::too_many_arguments,
    reason = "release identity, target, and progress remain explicit during staging"
)]
pub(super) fn stage_release_entry(
    icp_root: &Path,
    network: &str,
    local_replica: Option<&LocalReplicaTarget>,
    root_canister: &str,
    release_version: &str,
    entry: &ReleaseSetEntry,
    now_secs: u64,
    progress: &mut StageProgress,
) -> Result<(), Box<dyn std::error::Error>> {
    let started_at = Instant::now();
    let artifact_path = icp_root.join(&entry.artifact_relative_path);
    let wasm_module = read_release_artifact(&artifact_path)?;

    if wasm_module.len() as u64 != entry.payload_size_bytes {
        return Err(format!(
            "release artifact size drift for {}: manifest={} actual={} ({})",
            entry.role,
            entry.payload_size_bytes,
            wasm_module.len(),
            artifact_path.display()
        )
        .into());
    }

    let chunk_count = wasm_module.chunks(CANIC_WASM_CHUNK_BYTES).count();
    if chunk_count != entry.chunk_sha256_hex.len() {
        return Err(format!(
            "release chunk count drift for {}: manifest={} actual={} ({})",
            entry.role,
            entry.chunk_sha256_hex.len(),
            chunk_count,
            artifact_path.display()
        )
        .into());
    }
    let payload_hash = decode_hex(&entry.payload_sha256_hex)?;

    stage_release_manifest(
        icp_root,
        network,
        local_replica,
        root_canister,
        release_version,
        entry,
        now_secs,
        &payload_hash,
        wasm_module.len(),
    )?;

    prepare_release_chunks(
        icp_root,
        network,
        local_replica,
        root_canister,
        release_version,
        entry,
        &payload_hash,
        wasm_module.len(),
    )?;

    progress.start_entry(entry, chunk_count)?;
    publish_release_chunks(
        icp_root,
        network,
        local_replica,
        root_canister,
        release_version,
        entry,
        &wasm_module,
        progress,
    )?;
    progress.finish_entry(entry, chunk_count)?;
    progress.print_completed_entry(entry, started_at.elapsed());
    Ok(())
}

// Stage one approved manifest into root before any chunk preparation/upload begins.
#[expect(
    clippy::too_many_arguments,
    reason = "manifest identity and exact ICP target remain explicit at publication"
)]
fn stage_release_manifest(
    icp_root: &Path,
    network: &str,
    local_replica: Option<&LocalReplicaTarget>,
    root_canister: &str,
    release_version: &str,
    entry: &ReleaseSetEntry,
    now_secs: u64,
    payload_hash: &[u8],
    payload_size_bytes: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = format!(
        "(record {{ template_id = {}; role = {}; version = {}; payload_hash = {}; \
         payload_size_bytes = {} : nat64; store_binding = \"bootstrap\"; \
         chunking_mode = variant {{ Chunked }}; manifest_state = variant {{ Approved }}; \
         approved_at = opt ({} : nat64); created_at = {} : nat64 }})",
        idl_text(&entry.template_id),
        idl_text(&entry.role),
        idl_text(release_version),
        idl_blob(payload_hash),
        payload_size_bytes,
        now_secs,
        now_secs,
    );
    let _ = icp_call_on_network(
        icp_root,
        network,
        local_replica,
        root_canister,
        protocol::CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
        Some(&manifest),
        None,
    )?;
    Ok(())
}

// Prepare the root-local chunk set metadata before sending any chunk bytes.
#[expect(
    clippy::too_many_arguments,
    reason = "chunk identity and exact ICP target remain explicit at preparation"
)]
fn prepare_release_chunks(
    icp_root: &Path,
    network: &str,
    local_replica: Option<&LocalReplicaTarget>,
    root_canister: &str,
    release_version: &str,
    entry: &ReleaseSetEntry,
    payload_hash: &[u8],
    payload_size_bytes: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let chunk_hash_literals = entry
        .chunk_sha256_hex
        .iter()
        .map(|hash| {
            decode_hex(hash)
                .map(|bytes| idl_blob(&bytes))
                .map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?
        .join("; ");

    let prepare = format!(
        "(record {{ template_id = {}; version = {}; payload_hash = {}; \
         payload_size_bytes = {} : nat64; chunk_hashes = vec {{ {} }} }})",
        idl_text(&entry.template_id),
        idl_text(release_version),
        idl_blob(payload_hash),
        payload_size_bytes,
        chunk_hash_literals,
    );
    let _ = icp_call_on_network(
        icp_root,
        network,
        local_replica,
        root_canister,
        protocol::CANIC_TEMPLATE_PREPARE_ADMIN,
        Some(&prepare),
        None,
    )?;
    Ok(())
}

// Upload every prepared chunk and print live progress before and after each call.
#[expect(
    clippy::too_many_arguments,
    reason = "chunk identity, exact ICP target, and progress remain explicit during upload"
)]
fn publish_release_chunks(
    icp_root: &Path,
    network: &str,
    local_replica: Option<&LocalReplicaTarget>,
    root_canister: &str,
    release_version: &str,
    entry: &ReleaseSetEntry,
    wasm_module: &[u8],
    progress: &StageProgress,
) -> Result<(), Box<dyn std::error::Error>> {
    let chunk_count = wasm_module.chunks(CANIC_WASM_CHUNK_BYTES).count();
    for (chunk_index, chunk) in wasm_module.chunks(CANIC_WASM_CHUNK_BYTES).enumerate() {
        let request = format!(
            "(record {{ template_id = {}; version = {}; chunk_index = {} : nat32; bytes = {} }})",
            idl_text(&entry.template_id),
            idl_text(release_version),
            chunk_index,
            idl_blob(chunk),
        );
        let _ = icp_call_on_network(
            icp_root,
            network,
            local_replica,
            root_canister,
            protocol::CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
            Some(&request),
            None,
        )?;
        progress.update_entry(entry, chunk_index + 1, chunk_count)?;
    }
    Ok(())
}
