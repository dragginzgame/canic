use super::{
    TemplateManifestOps, TemplateManifestOpsError, WasmStoreGcExecutionStats, WasmStoreLimits,
    input_to_record,
};
use crate::{
    dto::template::{
        TemplateChunkInput, TemplateChunkResponse, TemplateChunkSetInfoResponse,
        TemplateChunkSetInput, TemplateChunkSetPrepareInput, TemplateManifestInput,
        TemplateManifestResponse, TemplateStagingStatusResponse, WasmStoreBootstrapDebugResponse,
        WasmStoreGcStatusResponse, WasmStoreStatusResponse, WasmStoreTemplateStatusResponse,
    },
    ids::{
        CanisterRole, TemplateChunkKey, TemplateChunkingMode, TemplateId, TemplateManifestState,
        TemplateReleaseKey, TemplateVersion, WasmStoreGcStatus,
    },
    storage::stable::template::{
        TemplateChunkRecord, TemplateChunkSetRecord, TemplateChunkSetStateStore,
        TemplateChunkStore, TemplateManifestRecord, TemplateManifestStateStore,
    },
};
use canic_core::__control_plane_core as cp_core;
use cp_core::{
    InternalError,
    cdk::{api::canister_self, structures::storable::Storable, utils::wasm::get_wasm_hash},
    format::byte_size,
    ops::ic::mgmt::MgmtOps,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

///
/// TemplateChunkedOps
///

pub struct TemplateChunkedOps;

impl TemplateChunkedOps {
    // Return staged-release status for every approved manifest in deterministic role order.
    #[must_use]
    pub fn approved_staging_status_responses() -> Vec<TemplateStagingStatusResponse> {
        let chunk_counts = TemplateChunkStore::count_by_release();
        let mut staged = TemplateManifestOps::approved_manifests_response()
            .into_iter()
            .map(|manifest| Self::staging_status_response(&manifest, &chunk_counts))
            .collect::<Vec<_>>();

        staged.sort_by(|left, right| left.role.cmp(&right.role));
        staged
    }

    // Return a root-owned bootstrap debug snapshot for the staged bootstrap role and release set.
    pub fn bootstrap_debug_response(
        bootstrap_role: &CanisterRole,
    ) -> Result<WasmStoreBootstrapDebugResponse, InternalError> {
        let staged = Self::approved_staging_status_responses();
        let bootstrap = staged
            .iter()
            .find(|entry| entry.role == *bootstrap_role)
            .cloned();
        let ready_for_bootstrap = Self::has_publishable_chunked_approved_for_role(bootstrap_role)?;

        Ok(WasmStoreBootstrapDebugResponse {
            ready_for_bootstrap,
            bootstrap,
            staged,
        })
    }

    // Return current occupied-byte and template-retention state for this local store.
    #[must_use]
    pub fn store_status_response(
        limits: WasmStoreLimits,
        headroom_bytes: Option<u64>,
        gc: WasmStoreGcStatus,
    ) -> WasmStoreStatusResponse {
        let manifests = TemplateManifestStateStore::export().entries;
        let chunk_sets = TemplateChunkSetStateStore::export();
        let occupied_store_bytes = TemplateManifestStateStore::occupied_bytes()
            + TemplateChunkSetStateStore::occupied_bytes()
            + TemplateChunkStore::occupied_bytes();
        let template_versions = projected_template_versions(&manifests, &chunk_sets);
        let remaining_store_bytes = limits.max_store_bytes.saturating_sub(occupied_store_bytes);
        let release_count = u32::try_from(
            template_versions
                .values()
                .map(std::collections::BTreeSet::len)
                .sum::<usize>(),
        )
        .unwrap_or(u32::MAX);
        let template_count = u32::try_from(template_versions.len()).unwrap_or(u32::MAX);
        let within_headroom =
            headroom_bytes.is_some_and(|threshold| remaining_store_bytes <= threshold);
        let mut templates = template_versions
            .into_iter()
            .map(|(template_id, versions)| WasmStoreTemplateStatusResponse {
                template_id,
                versions: u16::try_from(versions.len()).unwrap_or(u16::MAX),
            })
            .collect::<Vec<_>>();
        templates.sort_by(|left, right| left.template_id.cmp(&right.template_id));

        WasmStoreStatusResponse {
            gc: WasmStoreGcStatusResponse {
                mode: gc.mode,
                changed_at: gc.changed_at,
                prepared_at: gc.prepared_at,
                started_at: gc.started_at,
                completed_at: gc.completed_at,
                runs_completed: gc.runs_completed,
            },
            occupied_store_bytes,
            occupied_store_size: byte_size(occupied_store_bytes),
            max_store_bytes: limits.max_store_bytes,
            max_store_size: byte_size(limits.max_store_bytes),
            remaining_store_bytes,
            remaining_store_size: byte_size(remaining_store_bytes),
            headroom_bytes,
            headroom_size: headroom_bytes.map(byte_size),
            within_headroom,
            template_count,
            max_templates: limits.max_templates,
            release_count,
            max_template_versions_per_template: limits.max_template_versions_per_template,
            templates,
        }
    }

    // Return whether one approved chunked manifest is fully staged and ready for publication.
    pub fn has_publishable_chunked_approved_for_role(
        role: &CanisterRole,
    ) -> Result<bool, InternalError> {
        if !TemplateManifestOps::has_approved_for_role(role)? {
            return Ok(false);
        }

        let manifest = TemplateManifestOps::approved_for_role_response(role)?;

        if manifest.chunking_mode != TemplateChunkingMode::Chunked {
            return Ok(false);
        }

        Ok(Self::validate_staged_release(&manifest).is_ok())
    }

    // Return deterministic staged-chunk progress for one approved manifest.
    #[must_use]
    pub fn staging_status_response(
        manifest: &TemplateManifestResponse,
        chunk_counts: &BTreeMap<TemplateReleaseKey, u32>,
    ) -> TemplateStagingStatusResponse {
        let release =
            TemplateReleaseKey::new(manifest.template_id.clone(), manifest.version.clone());
        let chunk_set = TemplateChunkSetStateStore::get(&release);
        let expected_chunk_count = chunk_set.as_ref().map_or(0, |record| record.chunk_count);
        let stored_chunk_count = chunk_counts.get(&release).copied().unwrap_or(0);
        let publishable = manifest.chunking_mode == TemplateChunkingMode::Chunked
            && chunk_set.is_some()
            && stored_chunk_count == expected_chunk_count;

        TemplateStagingStatusResponse {
            role: manifest.role.clone(),
            template_id: manifest.template_id.clone(),
            version: manifest.version.clone(),
            store_binding: manifest.store_binding.clone(),
            chunking_mode: manifest.chunking_mode,
            payload_size_bytes: manifest.payload_size_bytes,
            payload_size: byte_size(manifest.payload_size_bytes),
            chunk_set_present: chunk_set.is_some(),
            expected_chunk_count,
            stored_chunk_count,
            publishable,
        }
    }

    // Replace the approved manifest for a local wasm store with capacity enforcement.
    pub fn replace_approved_in_store_from_input(
        input: TemplateManifestInput,
        limits: WasmStoreLimits,
    ) -> Result<(), InternalError> {
        let projected_manifests = projected_manifests_after_replace(&input);
        let projected_chunk_sets = TemplateChunkSetStateStore::export();
        let projected_bytes = manifest_store_bytes(&projected_manifests)
            + chunk_set_store_bytes(&projected_chunk_sets)
            + TemplateChunkStore::occupied_bytes();
        let projected_versions =
            projected_template_versions(&projected_manifests, &projected_chunk_sets);
        ensure_store_limits_from_versions(limits, projected_bytes, projected_versions)?;

        TemplateManifestOps::replace_approved_from_input(input);
        Ok(())
    }

    // Publish one complete chunk set into the local wasm store.
    pub fn publish_chunk_set_from_input(
        input: TemplateChunkSetInput,
        created_at: u64,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        let release = TemplateReleaseKey::new(input.template_id, input.version);
        if input.chunks.is_empty() {
            return Err(TemplateManifestOpsError::TemplateChunkSetEmpty(release).into());
        }

        let payload_size_bytes = input
            .chunks
            .iter()
            .map(|chunk| chunk.len() as u64)
            .sum::<u64>();
        if payload_size_bytes != input.payload_size_bytes {
            return Err(TemplateManifestOpsError::PayloadSizeMismatch(release).into());
        }

        let mut payload_hasher = Sha256::new();
        let mut chunk_hashes = Vec::with_capacity(input.chunks.len());

        for chunk in &input.chunks {
            payload_hasher.update(chunk);
            chunk_hashes.push(get_wasm_hash(chunk));
        }

        if payload_hasher.finalize().to_vec() != input.payload_hash {
            return Err(TemplateManifestOpsError::PayloadHashMismatch(release).into());
        }

        let info = Self::prepare_chunk_set_from_input(
            TemplateChunkSetPrepareInput {
                template_id: release.template_id.clone(),
                version: release.version.clone(),
                payload_hash: input.payload_hash,
                payload_size_bytes: input.payload_size_bytes,
                chunk_hashes,
            },
            created_at,
        )?;

        for (chunk_index, bytes) in input.chunks.into_iter().enumerate() {
            let chunk_index = u32::try_from(chunk_index)
                .map_err(|_| TemplateManifestOpsError::ChunkIndexOverflow(release.clone()))?;
            Self::publish_chunk_from_input(TemplateChunkInput {
                template_id: release.template_id.clone(),
                version: release.version.clone(),
                chunk_index,
                bytes,
            })?;
        }

        Ok(info)
    }

    // Publish one complete chunk set into a local store with capacity enforcement.
    #[cfg_attr(not(test), expect(dead_code))]
    pub fn publish_chunk_set_in_store_from_input(
        input: TemplateChunkSetInput,
        created_at: u64,
        limits: WasmStoreLimits,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        let release = TemplateReleaseKey::new(input.template_id.clone(), input.version.clone());
        if input.chunks.is_empty() {
            return Err(TemplateManifestOpsError::TemplateChunkSetEmpty(release).into());
        }

        let payload_size_bytes = input
            .chunks
            .iter()
            .map(|chunk| chunk.len() as u64)
            .sum::<u64>();
        if payload_size_bytes != input.payload_size_bytes {
            return Err(TemplateManifestOpsError::PayloadSizeMismatch(release).into());
        }

        let mut payload_hasher = Sha256::new();
        let mut chunk_hashes = Vec::with_capacity(input.chunks.len());

        for chunk in &input.chunks {
            payload_hasher.update(chunk);
            chunk_hashes.push(get_wasm_hash(chunk));
        }

        if payload_hasher.finalize().to_vec() != input.payload_hash {
            return Err(TemplateManifestOpsError::PayloadHashMismatch(release).into());
        }

        let chunk_count = u32::try_from(chunk_hashes.len())
            .map_err(|_| TemplateManifestOpsError::ChunkIndexOverflow(release.clone()))?;
        let projected_chunk_set = TemplateChunkSetRecord {
            payload_hash: input.payload_hash.clone(),
            payload_size_bytes: input.payload_size_bytes,
            chunk_count,
            chunk_hashes,
            created_at,
        };
        let projected_chunks = input
            .chunks
            .iter()
            .enumerate()
            .map(|(chunk_index, bytes)| {
                let chunk_index = u32::try_from(chunk_index)
                    .map_err(|_| TemplateManifestOpsError::ChunkIndexOverflow(release.clone()))?;
                Ok((
                    TemplateChunkKey::new(release.clone(), chunk_index),
                    TemplateChunkRecord {
                        bytes: bytes.clone(),
                    },
                ))
            })
            .collect::<Result<Vec<_>, InternalError>>()?;

        let projected_manifests = TemplateManifestStateStore::export().entries;
        let projected_chunk_sets = replace_chunk_set_entry(release, projected_chunk_set);
        let current_chunk_bytes = TemplateChunkStore::occupied_bytes();
        let replaced_chunk_bytes = projected_chunks
            .iter()
            .map(|(chunk_key, _)| TemplateChunkStore::entry_bytes(chunk_key).unwrap_or(0))
            .sum::<u64>();
        let inserted_chunk_bytes = projected_chunks
            .iter()
            .map(|(chunk_key, record)| chunk_entry_store_bytes(chunk_key, record))
            .sum::<u64>();
        let projected_bytes = TemplateManifestStateStore::occupied_bytes()
            + chunk_set_store_bytes(&projected_chunk_sets)
            + current_chunk_bytes
                .saturating_sub(replaced_chunk_bytes)
                .saturating_add(inserted_chunk_bytes);
        let projected_versions =
            projected_template_versions(&projected_manifests, &projected_chunk_sets);
        ensure_store_limits_from_versions(limits, projected_bytes, projected_versions)?;

        Self::publish_chunk_set_from_input(input, created_at)
    }

    // Prepare one chunk-set metadata record before chunk-by-chunk publication begins.
    pub fn prepare_chunk_set_from_input(
        input: TemplateChunkSetPrepareInput,
        created_at: u64,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        let release = TemplateReleaseKey::new(input.template_id, input.version);
        if input.chunk_hashes.is_empty() {
            return Err(TemplateManifestOpsError::TemplateChunkSetEmpty(release).into());
        }

        let chunk_count = u32::try_from(input.chunk_hashes.len())
            .map_err(|_| TemplateManifestOpsError::ChunkIndexOverflow(release.clone()))?;
        let info_record = TemplateChunkSetRecord {
            payload_hash: input.payload_hash,
            payload_size_bytes: input.payload_size_bytes,
            chunk_count,
            chunk_hashes: input.chunk_hashes,
            created_at,
        };

        TemplateChunkSetStateStore::upsert(release, info_record.clone());

        Ok(chunk_set_record_to_response(info_record))
    }

    // Prepare one chunk-set metadata record in a local store with capacity enforcement.
    pub fn prepare_chunk_set_in_store_from_input(
        input: TemplateChunkSetPrepareInput,
        created_at: u64,
        limits: WasmStoreLimits,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        let release = TemplateReleaseKey::new(input.template_id.clone(), input.version.clone());
        if input.chunk_hashes.is_empty() {
            return Err(TemplateManifestOpsError::TemplateChunkSetEmpty(release).into());
        }

        let chunk_count = u32::try_from(input.chunk_hashes.len())
            .map_err(|_| TemplateManifestOpsError::ChunkIndexOverflow(release.clone()))?;
        let info_record = TemplateChunkSetRecord {
            payload_hash: input.payload_hash,
            payload_size_bytes: input.payload_size_bytes,
            chunk_count,
            chunk_hashes: input.chunk_hashes,
            created_at,
        };

        let projected_manifests = TemplateManifestStateStore::export().entries;
        let projected_chunk_sets = replace_chunk_set_entry(release.clone(), info_record.clone());
        let projected_bytes = TemplateManifestStateStore::occupied_bytes()
            + chunk_set_store_bytes(&projected_chunk_sets)
            + TemplateChunkStore::occupied_bytes();
        let projected_versions =
            projected_template_versions(&projected_manifests, &projected_chunk_sets);
        ensure_store_limits_from_versions(limits, projected_bytes, projected_versions)?;

        TemplateChunkSetStateStore::upsert(release, info_record.clone());

        Ok(chunk_set_record_to_response(info_record))
    }

    // Publish one chunk into an already prepared local template release.
    pub fn publish_chunk_from_input(input: TemplateChunkInput) -> Result<(), InternalError> {
        let release = TemplateReleaseKey::new(input.template_id, input.version);
        let info = TemplateChunkSetStateStore::get(&release)
            .ok_or_else(|| TemplateManifestOpsError::TemplateChunkSetMissing(release.clone()))?;

        if input.chunk_index >= info.chunk_count {
            return Err(TemplateManifestOpsError::TemplateChunkIndexOutOfRange(
                release,
                input.chunk_index,
            )
            .into());
        }

        let expected_hash = &info.chunk_hashes[input.chunk_index as usize];
        let actual_hash = get_wasm_hash(&input.bytes);
        let chunk_key = TemplateChunkKey::new(release, input.chunk_index);

        if actual_hash != *expected_hash {
            return Err(TemplateManifestOpsError::TemplateChunkHashMismatch(chunk_key).into());
        }
        canic_core::perf!("publish_stage_validate_chunk");

        TemplateChunkStore::upsert(chunk_key, TemplateChunkRecord { bytes: input.bytes });
        canic_core::perf!("publish_stage_upsert_chunk");

        Ok(())
    }

    // Publish one chunk into a local store with capacity enforcement.
    pub fn publish_chunk_in_store_from_input(
        input: TemplateChunkInput,
        limits: WasmStoreLimits,
    ) -> Result<(), InternalError> {
        let release = TemplateReleaseKey::new(input.template_id.clone(), input.version.clone());
        let info = TemplateChunkSetStateStore::get(&release)
            .ok_or_else(|| TemplateManifestOpsError::TemplateChunkSetMissing(release.clone()))?;

        if input.chunk_index >= info.chunk_count {
            return Err(TemplateManifestOpsError::TemplateChunkIndexOutOfRange(
                release,
                input.chunk_index,
            )
            .into());
        }

        let expected_hash = &info.chunk_hashes[input.chunk_index as usize];
        let actual_hash = get_wasm_hash(&input.bytes);
        let chunk_key = TemplateChunkKey::new(release, input.chunk_index);

        if actual_hash != *expected_hash {
            return Err(TemplateManifestOpsError::TemplateChunkHashMismatch(chunk_key).into());
        }
        canic_core::perf!("publish_store_validate_chunk");

        // Manifest/template-version limits are fixed by the earlier manifest + prepare phases.
        // Publishing one chunk can only change the occupied-byte total for this store.
        let new_record = TemplateChunkRecord { bytes: input.bytes };
        let current_store_bytes = TemplateManifestStateStore::occupied_bytes()
            + TemplateChunkSetStateStore::occupied_bytes()
            + TemplateChunkStore::occupied_bytes();
        let existing_chunk_bytes = TemplateChunkStore::entry_bytes(&chunk_key).unwrap_or(0);
        let projected_bytes = current_store_bytes
            .saturating_sub(existing_chunk_bytes)
            .saturating_add(chunk_entry_store_bytes(&chunk_key, &new_record));
        canic_core::perf!("publish_store_project_capacity");

        if projected_bytes > limits.max_store_bytes {
            return Err(TemplateManifestOpsError::WasmStoreCapacityExceeded {
                projected_bytes,
                max_store_bytes: limits.max_store_bytes,
            }
            .into());
        }
        canic_core::perf!("publish_store_enforce_capacity");

        TemplateChunkStore::upsert(chunk_key, new_record);
        canic_core::perf!("publish_store_upsert_chunk");

        Ok(())
    }

    // Return deterministic chunk-set metadata for one template release.
    pub fn chunk_set_info_response(
        template_id: &TemplateId,
        version: &TemplateVersion,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        let release = TemplateReleaseKey::new(template_id.clone(), version.clone());
        let record = TemplateChunkSetStateStore::get(&release)
            .ok_or_else(|| TemplateManifestOpsError::TemplateChunkSetMissing(release.clone()))?;

        Ok(chunk_set_record_to_response(record))
    }

    // Return one deterministic chunk for one template release.
    pub fn chunk_response(
        template_id: &TemplateId,
        version: &TemplateVersion,
        chunk_index: u32,
    ) -> Result<TemplateChunkResponse, InternalError> {
        let release = TemplateReleaseKey::new(template_id.clone(), version.clone());
        let chunk_key = TemplateChunkKey::new(release, chunk_index);
        let record = TemplateChunkStore::get(&chunk_key)
            .ok_or_else(|| TemplateManifestOpsError::TemplateChunkMissing(chunk_key.clone()))?;

        Ok(TemplateChunkResponse {
            bytes: record.bytes,
        })
    }

    // Verify that one approved chunked manifest has a complete staged payload with matching hashes.
    pub fn validate_staged_release(
        manifest: &TemplateManifestResponse,
    ) -> Result<(), InternalError> {
        let info = Self::chunk_set_info_response(&manifest.template_id, &manifest.version)?;
        let release =
            TemplateReleaseKey::new(manifest.template_id.clone(), manifest.version.clone());

        if info.chunk_hashes.is_empty() {
            return Err(TemplateManifestOpsError::TemplateChunkSetEmpty(release).into());
        }

        let mut payload_hasher = Sha256::new();
        let mut payload_size_bytes = 0_u64;

        for (chunk_index, expected_hash) in info.chunk_hashes.iter().enumerate() {
            let chunk_index = u32::try_from(chunk_index)
                .map_err(|_| TemplateManifestOpsError::ChunkIndexOverflow(release.clone()))?;
            let response =
                Self::chunk_response(&manifest.template_id, &manifest.version, chunk_index)?;
            let actual_hash = get_wasm_hash(&response.bytes);
            let chunk_key = TemplateChunkKey::new(release.clone(), chunk_index);

            if &actual_hash != expected_hash {
                return Err(TemplateManifestOpsError::TemplateChunkHashMismatch(chunk_key).into());
            }

            payload_size_bytes = payload_size_bytes.saturating_add(response.bytes.len() as u64);
            payload_hasher.update(&response.bytes);
        }

        if payload_size_bytes != manifest.payload_size_bytes {
            return Err(TemplateManifestOpsError::PayloadSizeMismatch(release).into());
        }

        if payload_hasher.finalize().to_vec() != manifest.payload_hash {
            return Err(TemplateManifestOpsError::PayloadHashMismatch(release).into());
        }

        Ok(())
    }

    // Clear all local template metadata and chunk bytes for store-local GC execution.
    pub async fn execute_local_store_gc() -> Result<WasmStoreGcExecutionStats, InternalError> {
        let manifests = TemplateManifestStateStore::export().entries;
        let chunk_sets = TemplateChunkSetStateStore::export();
        let chunks = TemplateChunkStore::export();
        let stored_chunk_hashes = MgmtOps::stored_chunks(canister_self()).await?;
        let template_count =
            u32::try_from(projected_template_versions(&manifests, &chunk_sets).len())
                .unwrap_or(u32::MAX);
        let release_count = u32::try_from(
            projected_template_versions(&manifests, &chunk_sets)
                .values()
                .map(BTreeSet::len)
                .sum::<usize>(),
        )
        .unwrap_or(u32::MAX);
        let chunk_count = u32::try_from(chunks.len()).unwrap_or(u32::MAX);
        let chunk_store_hash_count = u32::try_from(stored_chunk_hashes.len()).unwrap_or(u32::MAX);
        let reclaimed_store_bytes = TemplateManifestStateStore::occupied_bytes()
            + TemplateChunkSetStateStore::occupied_bytes()
            + TemplateChunkStore::occupied_bytes();

        MgmtOps::clear_chunk_store(canister_self()).await?;
        TemplateManifestStateStore::clear();
        TemplateChunkSetStateStore::clear();
        TemplateChunkStore::clear();

        Ok(WasmStoreGcExecutionStats {
            reclaimed_store_bytes,
            cleared_template_count: template_count,
            cleared_release_count: release_count,
            cleared_chunk_count: chunk_count,
            cleared_chunk_store_hash_count: chunk_store_hash_count,
        })
    }
}

// Map one stored chunk-set record into the public metadata response.
fn chunk_set_record_to_response(record: TemplateChunkSetRecord) -> TemplateChunkSetInfoResponse {
    TemplateChunkSetInfoResponse {
        chunk_hashes: record.chunk_hashes,
    }
}

fn manifest_store_bytes(manifests: &[(TemplateReleaseKey, TemplateManifestRecord)]) -> u64 {
    manifests
        .iter()
        .map(|(template_id, record)| {
            (template_id.to_bytes().len() + record.to_bytes().len()) as u64
        })
        .sum::<u64>()
}

fn chunk_set_store_bytes(chunk_sets: &[(TemplateReleaseKey, TemplateChunkSetRecord)]) -> u64 {
    chunk_sets
        .iter()
        .map(|(release, record)| (release.to_bytes().len() + record.to_bytes().len()) as u64)
        .sum::<u64>()
}

fn ensure_store_limits_from_versions(
    limits: WasmStoreLimits,
    projected_bytes: u64,
    projected_versions: BTreeMap<TemplateId, BTreeSet<TemplateVersion>>,
) -> Result<(), InternalError> {
    if projected_bytes > limits.max_store_bytes {
        return Err(TemplateManifestOpsError::WasmStoreCapacityExceeded {
            projected_bytes,
            max_store_bytes: limits.max_store_bytes,
        }
        .into());
    }

    if let Some(max_templates) = limits.max_templates {
        let projected_templates = u32::try_from(projected_versions.len()).unwrap_or(u32::MAX);
        if projected_templates > max_templates {
            return Err(TemplateManifestOpsError::WasmStoreTemplateLimitExceeded {
                projected_templates,
                max_templates,
            }
            .into());
        }
    }

    if let Some(max_versions) = limits.max_template_versions_per_template {
        for (template_id, versions) in projected_versions {
            let projected_versions = u16::try_from(versions.len()).unwrap_or(u16::MAX);
            if projected_versions > max_versions {
                return Err(TemplateManifestOpsError::WasmStoreVersionLimitExceeded {
                    template_id,
                    projected_versions,
                    max_template_versions_per_template: max_versions,
                }
                .into());
            }
        }
    }

    Ok(())
}

fn projected_template_versions(
    manifests: &[(TemplateReleaseKey, TemplateManifestRecord)],
    chunk_sets: &[(TemplateReleaseKey, TemplateChunkSetRecord)],
) -> BTreeMap<TemplateId, BTreeSet<TemplateVersion>> {
    let mut template_versions = BTreeMap::<TemplateId, BTreeSet<TemplateVersion>>::new();

    for (release, _) in manifests {
        template_versions
            .entry(release.template_id.clone())
            .or_default()
            .insert(release.version.clone());
    }

    for (release, _) in chunk_sets {
        template_versions
            .entry(release.template_id.clone())
            .or_default()
            .insert(release.version.clone());
    }

    template_versions
}
fn projected_manifests_after_replace(
    input: &TemplateManifestInput,
) -> Vec<(TemplateReleaseKey, TemplateManifestRecord)> {
    let role = input.role.clone();
    let release = TemplateReleaseKey::new(input.template_id.clone(), input.version.clone());
    let mut manifests = TemplateManifestStateStore::export().entries;

    for (existing_release, existing) in &mut manifests {
        if existing.role != role {
            continue;
        }
        if *existing_release == release {
            continue;
        }
        if existing.manifest_state != TemplateManifestState::Approved {
            continue;
        }

        existing.manifest_state = TemplateManifestState::Deprecated;
    }

    let record = input_to_record(input.clone());
    if let Some(existing) = manifests
        .iter_mut()
        .find(|(existing_release, _)| *existing_release == release)
    {
        existing.1 = record;
    } else {
        manifests.push((release, record));
    }

    manifests
}

fn replace_chunk_set_entry(
    release: TemplateReleaseKey,
    record: TemplateChunkSetRecord,
) -> Vec<(TemplateReleaseKey, TemplateChunkSetRecord)> {
    let mut entries = TemplateChunkSetStateStore::export();

    if let Some(existing) = entries
        .iter_mut()
        .find(|(existing_release, _)| *existing_release == release)
    {
        existing.1 = record;
    } else {
        entries.push((release, record));
    }

    entries
}

fn chunk_entry_store_bytes(chunk_key: &TemplateChunkKey, record: &TemplateChunkRecord) -> u64 {
    (chunk_key.to_bytes().len() + 12 + record.bytes.len()) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{TemplateId, WasmStoreBinding};

    fn reset_store() {
        TemplateManifestStateStore::clear_for_test();
        TemplateChunkSetStateStore::clear_for_test();
        TemplateChunkStore::clear_for_test();
    }

    fn approved_manifest_input() -> TemplateManifestInput {
        TemplateManifestInput {
            template_id: TemplateId::new("embedded:app"),
            role: CanisterRole::new("app"),
            version: TemplateVersion::new("0.18.0"),
            payload_hash: vec![7; 32],
            payload_size_bytes: 32,
            store_binding: WasmStoreBinding::new("primary"),
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(42),
            created_at: 41,
        }
    }

    #[test]
    fn publish_chunk_in_store_rejects_incremental_capacity_overflow() {
        reset_store();

        let chunk_zero = vec![1_u8; 8];
        let chunk_one = vec![2_u8; 8];
        let payload_hash = get_wasm_hash(&[chunk_zero.clone(), chunk_one.clone()].concat());
        let release = TemplateReleaseKey::new(
            TemplateId::new("embedded:app"),
            TemplateVersion::new("0.18.0"),
        );

        TemplateChunkedOps::replace_approved_in_store_from_input(
            approved_manifest_input(),
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: None,
                max_template_versions_per_template: None,
            },
        )
        .unwrap();

        TemplateChunkedOps::prepare_chunk_set_in_store_from_input(
            TemplateChunkSetPrepareInput {
                template_id: release.template_id.clone(),
                version: release.version.clone(),
                payload_hash,
                payload_size_bytes: 16,
                chunk_hashes: vec![get_wasm_hash(&chunk_zero), get_wasm_hash(&chunk_one)],
            },
            77,
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: None,
                max_template_versions_per_template: None,
            },
        )
        .unwrap();

        TemplateChunkedOps::publish_chunk_in_store_from_input(
            TemplateChunkInput {
                template_id: release.template_id.clone(),
                version: release.version.clone(),
                chunk_index: 0,
                bytes: chunk_zero,
            },
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: None,
                max_template_versions_per_template: None,
            },
        )
        .unwrap();

        let current_store_bytes = TemplateManifestStateStore::occupied_bytes()
            + TemplateChunkSetStateStore::occupied_bytes()
            + TemplateChunkStore::occupied_bytes();
        let chunk_one_key = TemplateChunkKey::new(release.clone(), 1);
        let chunk_one_record = TemplateChunkRecord {
            bytes: chunk_one.clone(),
        };
        let exact_limit =
            current_store_bytes + chunk_entry_store_bytes(&chunk_one_key, &chunk_one_record) - 1;

        let err = TemplateChunkedOps::publish_chunk_in_store_from_input(
            TemplateChunkInput {
                template_id: release.template_id,
                version: release.version,
                chunk_index: 1,
                bytes: chunk_one,
            },
            WasmStoreLimits {
                max_store_bytes: exact_limit,
                max_templates: None,
                max_template_versions_per_template: None,
            },
        )
        .expect_err("second chunk should fail once its incremental bytes exceed the limit");

        assert!(err.to_string().contains("capacity exceeded"));
    }
}
