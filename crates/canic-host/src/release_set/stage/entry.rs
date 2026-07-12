//! Module: release_set::stage::entry
//!
//! Responsibility: validate and stage one built release artifact into root.
//! Does not own: artifact construction, ICP target resolution, or bootstrap sequencing.
//! Boundary: maps the release manifest into canonical control-plane request DTOs.

use crate::{
    icp::LocalReplicaTarget,
    release_set::{
        ReleaseSetEntry,
        stage::{
            artifact::{read_release_artifact, resolve_release_artifact_path},
            call::icp_call_on_network,
            progress::StageProgress,
        },
    },
};
use std::{path::Path, time::Instant};

use canic_control_plane::{
    dto::template::{TemplateChunkInput, TemplateChunkSetPrepareInput, TemplateManifestInput},
    ids::{
        CanisterRole, TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion,
        WasmStoreBinding,
    },
};
use canic_core::{
    CANIC_WASM_CHUNK_BYTES,
    cdk::utils::hash::{decode_hex, wasm_hash},
    protocol,
};

struct StagedReleaseIdentity {
    template_id: TemplateId,
    role: CanisterRole,
    version: TemplateVersion,
}

impl StagedReleaseIdentity {
    fn new(entry: &ReleaseSetEntry, release_version: &str) -> Self {
        Self {
            template_id: TemplateId::owned(entry.template_id.clone()),
            role: CanisterRole::owned(entry.role.clone()),
            version: TemplateVersion::owned(release_version.to_string()),
        }
    }
}

struct ValidatedReleaseArtifact {
    bytes: Vec<u8>,
    payload_hash: Vec<u8>,
    chunk_hashes: Vec<Vec<u8>>,
}

impl ValidatedReleaseArtifact {
    fn read(path: &Path, entry: &ReleaseSetEntry) -> Result<Self, Box<dyn std::error::Error>> {
        let bytes = read_release_artifact(path)?;
        Self::validate(path, entry, bytes)
    }

    fn validate(
        path: &Path,
        entry: &ReleaseSetEntry,
        bytes: Vec<u8>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let actual_size = u64::try_from(bytes.len())?;

        if actual_size != entry.payload_size_bytes {
            return Err(format!(
                "release artifact size drift for {}: manifest={} actual={} ({})",
                entry.role,
                entry.payload_size_bytes,
                bytes.len(),
                path.display()
            )
            .into());
        }

        let payload_hash = wasm_hash(&bytes);
        let declared_payload_hash = decode_hex(&entry.payload_sha256_hex)?;
        if payload_hash != declared_payload_hash {
            return Err(format!(
                "release payload hash drift for {} ({})",
                entry.role,
                path.display()
            )
            .into());
        }

        let chunk_hashes = bytes
            .chunks(CANIC_WASM_CHUNK_BYTES)
            .map(wasm_hash)
            .collect::<Vec<_>>();
        if chunk_hashes.len() != entry.chunk_sha256_hex.len() {
            return Err(format!(
                "release chunk count drift for {}: manifest={} actual={} ({})",
                entry.role,
                entry.chunk_sha256_hex.len(),
                chunk_hashes.len(),
                path.display()
            )
            .into());
        }

        for (chunk_index, (actual_hash, declared_hash)) in
            chunk_hashes.iter().zip(&entry.chunk_sha256_hex).enumerate()
        {
            if actual_hash != &decode_hex(declared_hash)? {
                return Err(format!(
                    "release chunk hash drift for {} at index {} ({})",
                    entry.role,
                    chunk_index,
                    path.display()
                )
                .into());
            }
        }

        Ok(Self {
            bytes,
            payload_hash,
            chunk_hashes,
        })
    }
}

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
    let artifact_path = resolve_release_artifact_path(icp_root, &entry.artifact_relative_path)?;
    let artifact = ValidatedReleaseArtifact::read(&artifact_path, entry)?;
    let chunk_count = artifact.chunk_hashes.len();
    let identity = StagedReleaseIdentity::new(entry, release_version);

    stage_release_manifest(
        icp_root,
        network,
        local_replica,
        root_canister,
        &identity,
        now_secs,
        &artifact,
    )?;

    prepare_release_chunks(
        icp_root,
        network,
        local_replica,
        root_canister,
        &identity,
        &artifact,
    )?;

    progress.start_entry(entry, chunk_count)?;
    publish_release_chunks(
        icp_root,
        network,
        local_replica,
        root_canister,
        &identity,
        entry,
        &artifact.bytes,
        progress,
    )?;
    progress.finish_entry(entry, chunk_count)?;
    progress.print_completed_entry(entry, started_at.elapsed());
    Ok(())
}

// Stage one approved manifest into root before any chunk preparation/upload begins.
fn stage_release_manifest(
    icp_root: &Path,
    network: &str,
    local_replica: Option<&LocalReplicaTarget>,
    root_canister: &str,
    identity: &StagedReleaseIdentity,
    now_secs: u64,
    artifact: &ValidatedReleaseArtifact,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = TemplateManifestInput {
        template_id: identity.template_id.clone(),
        role: identity.role.clone(),
        version: identity.version.clone(),
        payload_hash: artifact.payload_hash.clone(),
        payload_size_bytes: u64::try_from(artifact.bytes.len())?,
        store_binding: WasmStoreBinding::new("bootstrap"),
        chunking_mode: TemplateChunkingMode::Chunked,
        manifest_state: TemplateManifestState::Approved,
        approved_at: Some(now_secs),
        created_at: now_secs,
    };
    let argument = candid::encode_one(&manifest)?;
    let _ = icp_call_on_network(
        icp_root,
        network,
        local_replica,
        root_canister,
        protocol::CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
        Some(&argument),
        None,
    )?;
    Ok(())
}

// Prepare the root-local chunk set metadata before sending any chunk bytes.
fn prepare_release_chunks(
    icp_root: &Path,
    network: &str,
    local_replica: Option<&LocalReplicaTarget>,
    root_canister: &str,
    identity: &StagedReleaseIdentity,
    artifact: &ValidatedReleaseArtifact,
) -> Result<(), Box<dyn std::error::Error>> {
    let prepare = TemplateChunkSetPrepareInput {
        template_id: identity.template_id.clone(),
        version: identity.version.clone(),
        payload_hash: artifact.payload_hash.clone(),
        payload_size_bytes: u64::try_from(artifact.bytes.len())?,
        chunk_hashes: artifact.chunk_hashes.clone(),
    };
    let argument = candid::encode_one(&prepare)?;
    let _ = icp_call_on_network(
        icp_root,
        network,
        local_replica,
        root_canister,
        protocol::CANIC_TEMPLATE_PREPARE_ADMIN,
        Some(&argument),
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
    identity: &StagedReleaseIdentity,
    entry: &ReleaseSetEntry,
    artifact_bytes: &[u8],
    progress: &StageProgress,
) -> Result<(), Box<dyn std::error::Error>> {
    let chunk_count = artifact_bytes.chunks(CANIC_WASM_CHUNK_BYTES).count();
    for (chunk_index, chunk) in artifact_bytes.chunks(CANIC_WASM_CHUNK_BYTES).enumerate() {
        let request = TemplateChunkInput {
            template_id: identity.template_id.clone(),
            version: identity.version.clone(),
            chunk_index: u32::try_from(chunk_index)?,
            bytes: chunk.to_vec(),
        };
        let argument = candid::encode_one(&request)?;
        let _ = icp_call_on_network(
            icp_root,
            network,
            local_replica,
            root_canister,
            protocol::CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
            Some(&argument),
            None,
        )?;
        progress.update_entry(entry, chunk_index + 1, chunk_count)?;
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::cdk::utils::hash::wasm_hash_hex;

    fn matching_release_entry(bytes: &[u8]) -> ReleaseSetEntry {
        ReleaseSetEntry {
            role: "app".to_string(),
            template_id: "embedded:app".to_string(),
            artifact_relative_path: "app/app.wasm.gz".to_string(),
            payload_size_bytes: u64::try_from(bytes.len()).expect("artifact size fits u64"),
            payload_sha256_hex: wasm_hash_hex(bytes),
            chunk_size_bytes: u64::try_from(CANIC_WASM_CHUNK_BYTES).expect("chunk size fits u64"),
            chunk_sha256_hex: bytes
                .chunks(CANIC_WASM_CHUNK_BYTES)
                .map(wasm_hash_hex)
                .collect(),
        }
    }

    #[test]
    fn release_artifact_validation_accepts_exact_manifest() {
        let bytes = b"artifact bytes";
        let entry = matching_release_entry(bytes);

        let artifact = ValidatedReleaseArtifact::validate(
            Path::new("artifact.wasm.gz"),
            &entry,
            bytes.to_vec(),
        )
        .expect("matching artifact validates");

        assert_eq!(artifact.bytes, bytes);
        assert_eq!(artifact.payload_hash, wasm_hash(bytes));
        assert_eq!(artifact.chunk_hashes, vec![wasm_hash(bytes)]);
    }

    #[test]
    fn release_artifact_validation_rejects_payload_hash_drift() {
        let bytes = b"artifact bytes";
        let mut entry = matching_release_entry(bytes);
        entry.payload_sha256_hex = wasm_hash_hex(b"different bytes");

        assert!(
            ValidatedReleaseArtifact::validate(
                Path::new("artifact.wasm.gz"),
                &entry,
                bytes.to_vec()
            )
            .is_err(),
            "payload hash drift must reject"
        );
    }

    #[test]
    fn release_artifact_validation_rejects_chunk_hash_drift() {
        let bytes = b"artifact bytes";
        let mut entry = matching_release_entry(bytes);
        entry.chunk_sha256_hex[0] = wasm_hash_hex(b"different chunk");

        assert!(
            ValidatedReleaseArtifact::validate(
                Path::new("artifact.wasm.gz"),
                &entry,
                bytes.to_vec()
            )
            .is_err(),
            "chunk hash drift must reject"
        );
    }

    #[test]
    fn maximum_chunk_binary_argument_fits_endpoint_limit_and_round_trips() {
        let request = TemplateChunkInput {
            template_id: TemplateId::new("embedded:app"),
            version: TemplateVersion::new("test-version"),
            chunk_index: 0,
            bytes: vec![0xA5; CANIC_WASM_CHUNK_BYTES],
        };

        let argument = candid::encode_one(&request).expect("encode chunk request");
        let decoded = candid::decode_one::<TemplateChunkInput>(&argument)
            .expect("decode chunk request through endpoint type");

        assert!(argument.len() <= CANIC_WASM_CHUNK_BYTES + 64 * 1024);
        assert_eq!(decoded, request);
    }
}
