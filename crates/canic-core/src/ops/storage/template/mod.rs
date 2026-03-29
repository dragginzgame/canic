use crate::{
    InternalError,
    cdk::structures::storable::Storable,
    cdk::utils::wasm::get_wasm_hash,
    dto::template::{
        TemplateChunkInput, TemplateChunkResponse, TemplateChunkSetInfoResponse,
        TemplateChunkSetInput, TemplateChunkSetPrepareInput, TemplateManifestInput,
        TemplateManifestResponse, WasmStoreCatalogEntryResponse, WasmStoreGcStatusResponse,
        WasmStoreStatusResponse, WasmStoreTemplateStatusResponse,
    },
    ids::{
        CanisterRole, TemplateChunkKey, TemplateId, TemplateManifestState, TemplateReleaseKey,
        TemplateVersion, WasmStoreGcStatus,
    },
    ops::{OpsError, ic::mgmt::MgmtOps, storage::StorageOpsError},
    storage::stable::template::{
        TemplateChunkRecord, TemplateChunkSetRecord, TemplateChunkSetStateStore,
        TemplateChunkStore, TemplateManifestRecord, TemplateManifestStateStore,
    },
};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error as ThisError;

///
/// TemplateManifestOpsError
///

#[derive(Debug, ThisError)]
pub enum TemplateManifestOpsError {
    #[error("approved manifest missing for '{0}'")]
    ApprovedManifestMissing(CanisterRole),

    #[error("multiple approved manifests for '{0}'")]
    ApprovedManifestConflict(CanisterRole),

    #[error("chunk set missing for '{0}'")]
    TemplateChunkSetMissing(TemplateReleaseKey),

    #[error("chunk missing for '{0}'")]
    TemplateChunkMissing(TemplateChunkKey),

    #[error("chunk set '{0}' must contain at least one chunk")]
    TemplateChunkSetEmpty(TemplateReleaseKey),

    #[error("chunk set '{0}' payload hash mismatch")]
    PayloadHashMismatch(TemplateReleaseKey),

    #[error("chunk set '{0}' payload size mismatch")]
    PayloadSizeMismatch(TemplateReleaseKey),

    #[error("chunk set '{0}' exceeds chunk index bounds")]
    ChunkIndexOverflow(TemplateReleaseKey),

    #[error("chunk index {1} out of range for '{0}'")]
    TemplateChunkIndexOutOfRange(TemplateReleaseKey, u32),

    #[error("chunk '{0}' hash mismatch")]
    TemplateChunkHashMismatch(TemplateChunkKey),

    #[error("wasm store capacity exceeded: bytes {projected_bytes} > {max_store_bytes}")]
    WasmStoreCapacityExceeded {
        projected_bytes: u64,
        max_store_bytes: u64,
    },

    #[error("wasm store template count exceeded: {projected_templates} > {max_templates}")]
    WasmStoreTemplateLimitExceeded {
        projected_templates: u32,
        max_templates: u32,
    },

    #[error(
        "wasm store version retention exceeded for '{template_id}': {projected_versions} > {max_template_versions_per_template}"
    )]
    WasmStoreVersionLimitExceeded {
        template_id: TemplateId,
        projected_versions: u16,
        max_template_versions_per_template: u16,
    },
}

impl From<TemplateManifestOpsError> for InternalError {
    fn from(err: TemplateManifestOpsError) -> Self {
        OpsError::from(StorageOpsError::from(err)).into()
    }
}

///
/// TemplateManifestOps
///

pub struct TemplateManifestOps;

///
/// WasmStoreLimits
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WasmStoreLimits {
    pub max_store_bytes: u64,
    pub max_templates: Option<u32>,
    pub max_template_versions_per_template: Option<u16>,
}

///
/// WasmStoreGcExecutionStats
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WasmStoreGcExecutionStats {
    pub reclaimed_store_bytes: u64,
    pub cleared_template_count: u32,
    pub cleared_release_count: u32,
    pub cleared_chunk_count: u32,
    pub cleared_chunk_store_hash_count: u32,
}

impl TemplateManifestOps {
    // Return all currently approved manifests in deterministic order.
    #[must_use]
    pub fn approved_manifests_response() -> Vec<TemplateManifestResponse> {
        let mut manifests = TemplateManifestStateStore::export()
            .entries
            .into_iter()
            .filter_map(|(release, record)| {
                (record.manifest_state == TemplateManifestState::Approved)
                    .then(|| record_to_response(release, record))
            })
            .collect::<Vec<_>>();

        manifests.sort_by(|left, right| left.role.cmp(&right.role));
        manifests
    }

    // Return the approved manifest catalog in a store-safe response shape.
    #[must_use]
    pub fn approved_catalog_response() -> Vec<WasmStoreCatalogEntryResponse> {
        Self::approved_manifests_response()
            .into_iter()
            .map(|manifest| WasmStoreCatalogEntryResponse {
                role: manifest.role,
                template_id: manifest.template_id,
                version: manifest.version,
                payload_hash: manifest.payload_hash,
                payload_size_bytes: manifest.payload_size_bytes,
            })
            .collect()
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
        let chunks = TemplateChunkStore::export();
        let occupied_store_bytes = occupied_store_bytes(&manifests, &chunk_sets, &chunks);
        let template_versions = projected_template_versions(&manifests, &chunk_sets, &chunks);
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
            max_store_bytes: limits.max_store_bytes,
            remaining_store_bytes,
            headroom_bytes,
            within_headroom,
            template_count,
            max_templates: limits.max_templates,
            release_count,
            max_template_versions_per_template: limits.max_template_versions_per_template,
            templates,
        }
    }

    // Return the single approved manifest for a role or an explicit conflict error.
    pub fn approved_for_role_response(
        role: &CanisterRole,
    ) -> Result<TemplateManifestResponse, InternalError> {
        let approved = TemplateManifestStateStore::export()
            .entries
            .into_iter()
            .filter(|(_, record)| {
                record.role == *role && record.manifest_state == TemplateManifestState::Approved
            })
            .collect::<Vec<_>>();

        match approved.as_slice() {
            [] => Err(TemplateManifestOpsError::ApprovedManifestMissing(role.clone()).into()),
            [(release, record)] => Ok(record_to_response(release.clone(), record.clone())),
            _ => Err(TemplateManifestOpsError::ApprovedManifestConflict(role.clone()).into()),
        }
    }

    // Return whether exactly one approved manifest exists for this role.
    pub fn has_approved_for_role(role: &CanisterRole) -> Result<bool, InternalError> {
        let approved_count = TemplateManifestStateStore::export()
            .entries
            .into_iter()
            .filter(|(_, record)| {
                record.role == *role && record.manifest_state == TemplateManifestState::Approved
            })
            .count();

        match approved_count {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(TemplateManifestOpsError::ApprovedManifestConflict(role.clone()).into()),
        }
    }

    // Replace the approved manifest for a role while deprecating older approved entries.
    pub fn replace_approved_from_input(input: TemplateManifestInput) {
        let role = input.role.clone();
        let release = TemplateReleaseKey::new(input.template_id.clone(), input.version.clone());

        for (existing_release, mut existing) in TemplateManifestStateStore::export().entries {
            if existing.role != role {
                continue;
            }
            if existing_release == release {
                continue;
            }
            if existing.manifest_state != TemplateManifestState::Approved {
                continue;
            }

            existing.manifest_state = TemplateManifestState::Deprecated;
            TemplateManifestStateStore::upsert(existing_release, existing);
        }

        TemplateManifestStateStore::upsert(release, input_to_record(input));
    }

    // Replace the approved manifest for a local wasm store with capacity enforcement.
    pub fn replace_approved_in_store_from_input(
        input: TemplateManifestInput,
        limits: WasmStoreLimits,
    ) -> Result<(), InternalError> {
        let projected_manifests = projected_manifests_after_replace(&input);
        let projected_chunk_sets = TemplateChunkSetStateStore::export();
        let projected_chunks = TemplateChunkStore::export();
        ensure_store_limits(
            limits,
            &projected_manifests,
            &projected_chunk_sets,
            &projected_chunks,
        )?;

        Self::replace_approved_from_input(input);
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

        let mut payload = Vec::new();
        let mut chunk_hashes = Vec::with_capacity(input.chunks.len());

        for chunk in &input.chunks {
            payload.extend_from_slice(chunk);
            chunk_hashes.push(get_wasm_hash(chunk));
        }

        if get_wasm_hash(&payload) != input.payload_hash {
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

        let mut payload = Vec::new();
        let mut chunk_hashes = Vec::with_capacity(input.chunks.len());

        for chunk in &input.chunks {
            payload.extend_from_slice(chunk);
            chunk_hashes.push(get_wasm_hash(chunk));
        }

        if get_wasm_hash(&payload) != input.payload_hash {
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
        let projected_chunks = replace_chunk_entries(projected_chunks);
        ensure_store_limits(
            limits,
            &projected_manifests,
            &projected_chunk_sets,
            &projected_chunks,
        )?;

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
        let projected_chunks = TemplateChunkStore::export();
        ensure_store_limits(
            limits,
            &projected_manifests,
            &projected_chunk_sets,
            &projected_chunks,
        )?;

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

        let expected_hash = info.chunk_hashes[input.chunk_index as usize].clone();
        let actual_hash = get_wasm_hash(&input.bytes);
        let chunk_key = TemplateChunkKey::new(release, input.chunk_index);

        if actual_hash != expected_hash {
            return Err(TemplateManifestOpsError::TemplateChunkHashMismatch(chunk_key).into());
        }

        TemplateChunkStore::upsert(chunk_key, TemplateChunkRecord { bytes: input.bytes });

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

        let expected_hash = info.chunk_hashes[input.chunk_index as usize].clone();
        let actual_hash = get_wasm_hash(&input.bytes);
        let chunk_key = TemplateChunkKey::new(release, input.chunk_index);

        if actual_hash != expected_hash {
            return Err(TemplateManifestOpsError::TemplateChunkHashMismatch(chunk_key).into());
        }

        let projected_manifests = TemplateManifestStateStore::export().entries;
        let projected_chunk_sets = TemplateChunkSetStateStore::export();
        let projected_chunks = replace_chunk_entries(vec![(
            chunk_key.clone(),
            TemplateChunkRecord {
                bytes: input.bytes.clone(),
            },
        )]);
        ensure_store_limits(
            limits,
            &projected_manifests,
            &projected_chunk_sets,
            &projected_chunks,
        )?;

        TemplateChunkStore::upsert(chunk_key, TemplateChunkRecord { bytes: input.bytes });

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

    // Clear all local template metadata and chunk bytes for store-local GC execution.
    pub async fn execute_local_store_gc() -> Result<WasmStoreGcExecutionStats, InternalError> {
        let manifests = TemplateManifestStateStore::export().entries;
        let chunk_sets = TemplateChunkSetStateStore::export();
        let chunks = TemplateChunkStore::export();
        let stored_chunk_hashes = MgmtOps::stored_chunks(crate::cdk::api::canister_self()).await?;
        let template_count =
            u32::try_from(projected_template_versions(&manifests, &chunk_sets, &chunks).len())
                .unwrap_or(u32::MAX);
        let release_count = u32::try_from(
            projected_template_versions(&manifests, &chunk_sets, &chunks)
                .values()
                .map(BTreeSet::len)
                .sum::<usize>(),
        )
        .unwrap_or(u32::MAX);
        let chunk_count = u32::try_from(chunks.len()).unwrap_or(u32::MAX);
        let chunk_store_hash_count = u32::try_from(stored_chunk_hashes.len()).unwrap_or(u32::MAX);
        let reclaimed_store_bytes = occupied_store_bytes(&manifests, &chunk_sets, &chunks);

        MgmtOps::clear_chunk_store(crate::cdk::api::canister_self()).await?;
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

// Map a manifest input DTO into the authoritative stored record.
fn input_to_record(input: TemplateManifestInput) -> TemplateManifestRecord {
    TemplateManifestRecord {
        role: input.role,
        version: input.version,
        payload_hash: input.payload_hash,
        payload_size_bytes: input.payload_size_bytes,
        store_binding: input.store_binding,
        chunking_mode: input.chunking_mode,
        manifest_state: input.manifest_state,
        approved_at: input.approved_at,
        created_at: input.created_at,
    }
}

// Map a stored manifest record into the public response shape.
fn record_to_response(
    release: TemplateReleaseKey,
    record: TemplateManifestRecord,
) -> TemplateManifestResponse {
    TemplateManifestResponse {
        template_id: release.template_id,
        role: record.role,
        version: release.version,
        payload_hash: record.payload_hash,
        payload_size_bytes: record.payload_size_bytes,
        store_binding: record.store_binding,
        chunking_mode: record.chunking_mode,
        manifest_state: record.manifest_state,
        approved_at: record.approved_at,
        created_at: record.created_at,
    }
}

// Map one stored chunk-set record into the public metadata response.
fn chunk_set_record_to_response(record: TemplateChunkSetRecord) -> TemplateChunkSetInfoResponse {
    TemplateChunkSetInfoResponse {
        chunk_hashes: record.chunk_hashes,
    }
}

fn occupied_store_bytes(
    manifests: &[(TemplateReleaseKey, TemplateManifestRecord)],
    chunk_sets: &[(TemplateReleaseKey, TemplateChunkSetRecord)],
    chunks: &[(TemplateChunkKey, TemplateChunkRecord)],
) -> u64 {
    let manifest_bytes = manifests
        .iter()
        .map(|(template_id, record)| {
            (template_id.to_bytes().len() + record.to_bytes().len()) as u64
        })
        .sum::<u64>();
    let chunk_set_bytes = chunk_sets
        .iter()
        .map(|(release, record)| (release.to_bytes().len() + record.to_bytes().len()) as u64)
        .sum::<u64>();
    let chunk_bytes = chunks
        .iter()
        .map(|(chunk_key, record)| (chunk_key.to_bytes().len() + record.to_bytes().len()) as u64)
        .sum::<u64>();

    manifest_bytes + chunk_set_bytes + chunk_bytes
}

fn ensure_store_limits(
    limits: WasmStoreLimits,
    manifests: &[(TemplateReleaseKey, TemplateManifestRecord)],
    chunk_sets: &[(TemplateReleaseKey, TemplateChunkSetRecord)],
    chunks: &[(TemplateChunkKey, TemplateChunkRecord)],
) -> Result<(), InternalError> {
    let projected_bytes = occupied_store_bytes(manifests, chunk_sets, chunks);
    if projected_bytes > limits.max_store_bytes {
        return Err(TemplateManifestOpsError::WasmStoreCapacityExceeded {
            projected_bytes,
            max_store_bytes: limits.max_store_bytes,
        }
        .into());
    }

    let projected_versions = projected_template_versions(manifests, chunk_sets, chunks);

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
    chunks: &[(TemplateChunkKey, TemplateChunkRecord)],
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

    for (chunk_key, _) in chunks {
        template_versions
            .entry(chunk_key.release.template_id.clone())
            .or_default()
            .insert(chunk_key.release.version.clone());
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

fn replace_chunk_entries(
    replacements: Vec<(TemplateChunkKey, TemplateChunkRecord)>,
) -> Vec<(TemplateChunkKey, TemplateChunkRecord)> {
    let mut entries = TemplateChunkStore::export();

    for (chunk_key, record) in replacements {
        if let Some(existing) = entries
            .iter_mut()
            .find(|(existing_key, _)| *existing_key == chunk_key)
        {
            existing.1 = record;
        } else {
            entries.push((chunk_key, record));
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids::{TemplateChunkingMode, TemplateVersion, WasmStoreBinding, WasmStoreGcMode},
        storage::stable::template::{TemplateChunkSetStateStore, TemplateChunkStore},
    };

    fn approved_input(template_id: &'static str, role: &'static str) -> TemplateManifestInput {
        TemplateManifestInput {
            template_id: TemplateId::new(template_id),
            role: CanisterRole::new(role),
            version: TemplateVersion::new("0.18.0"),
            payload_hash: vec![1; 32],
            payload_size_bytes: 512,
            store_binding: WasmStoreBinding::new("primary"),
            chunking_mode: TemplateChunkingMode::Inline,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(10),
            created_at: 9,
        }
    }

    fn chunk_set_input() -> TemplateChunkSetInput {
        let chunks = vec![vec![1, 2, 3], vec![4, 5]];
        let payload = chunks.concat();

        TemplateChunkSetInput {
            template_id: TemplateId::new("embedded:app"),
            version: TemplateVersion::new("0.18.0"),
            payload_hash: get_wasm_hash(&payload),
            payload_size_bytes: payload.len() as u64,
            chunks,
        }
    }

    fn store_limits(max_store_bytes: u64) -> WasmStoreLimits {
        WasmStoreLimits {
            max_store_bytes,
            max_templates: None,
            max_template_versions_per_template: None,
        }
    }

    fn reset_store() {
        TemplateManifestStateStore::clear_for_test();
        TemplateChunkSetStateStore::clear_for_test();
        TemplateChunkStore::clear_for_test();
    }

    fn approved_input_with_version(
        template_id: &'static str,
        role: &'static str,
        version: &'static str,
    ) -> TemplateManifestInput {
        let mut input = approved_input(template_id, role);
        input.version = TemplateVersion::new(version);
        input
    }

    #[test]
    fn replace_approved_keeps_one_approved_manifest_per_role() {
        reset_store();

        TemplateManifestOps::replace_approved_from_input(approved_input("one", "app"));
        TemplateManifestOps::replace_approved_from_input(approved_input("two", "app"));

        let manifests = TemplateManifestStateStore::export()
            .entries
            .into_iter()
            .map(|(template_id, record)| record_to_response(template_id, record))
            .collect::<Vec<_>>();
        let approved = manifests
            .iter()
            .filter(|entry| {
                entry.role == CanisterRole::new("app")
                    && entry.manifest_state == TemplateManifestState::Approved
            })
            .count();

        assert_eq!(approved, 1);
        assert_eq!(
            TemplateManifestOps::approved_for_role_response(&CanisterRole::new("app"))
                .unwrap()
                .template_id,
            TemplateId::new("two")
        );
    }

    #[test]
    fn has_approved_for_role_reports_presence() {
        reset_store();

        assert!(!TemplateManifestOps::has_approved_for_role(&CanisterRole::new("app")).unwrap());

        TemplateManifestOps::replace_approved_from_input(approved_input("one", "app"));

        assert!(TemplateManifestOps::has_approved_for_role(&CanisterRole::new("app")).unwrap());
    }

    #[test]
    fn publish_chunk_set_stores_info_and_chunks() {
        reset_store();

        let info =
            TemplateManifestOps::publish_chunk_set_from_input(chunk_set_input(), 77).unwrap();

        assert_eq!(info.chunk_hashes.len(), 2);

        let chunk = TemplateManifestOps::chunk_response(
            &TemplateId::new("embedded:app"),
            &TemplateVersion::new("0.18.0"),
            1,
        )
        .unwrap();

        assert_eq!(chunk.bytes, vec![4, 5]);
    }

    #[test]
    fn prepare_then_publish_chunk_rejects_hash_mismatch() {
        reset_store();

        let payload = vec![1_u8, 2, 3];
        TemplateManifestOps::prepare_chunk_set_from_input(
            TemplateChunkSetPrepareInput {
                template_id: TemplateId::new("embedded:app"),
                version: TemplateVersion::new("0.18.0"),
                payload_hash: get_wasm_hash(&payload),
                payload_size_bytes: payload.len() as u64,
                chunk_hashes: vec![get_wasm_hash(&payload)],
            },
            77,
        )
        .unwrap();

        let err = TemplateManifestOps::publish_chunk_from_input(TemplateChunkInput {
            template_id: TemplateId::new("embedded:app"),
            version: TemplateVersion::new("0.18.0"),
            chunk_index: 0,
            bytes: vec![9, 9, 9],
        })
        .expect_err("mismatched chunk hash must fail");

        assert!(err.to_string().contains("hash mismatch"));
    }

    #[test]
    fn store_capacity_rejects_chunk_set_that_exceeds_limit() {
        reset_store();

        TemplateManifestOps::replace_approved_in_store_from_input(
            approved_input("embedded:app", "app"),
            store_limits(10_000),
        )
        .unwrap();

        let err = TemplateManifestOps::publish_chunk_set_in_store_from_input(
            chunk_set_input(),
            77,
            store_limits(32),
        )
        .expect_err("chunk set should fail once projected store bytes exceed the limit");

        assert!(err.to_string().contains("capacity exceeded"));
    }

    #[test]
    fn store_capacity_rejects_manifest_update_that_exceeds_limit() {
        reset_store();

        let err = TemplateManifestOps::replace_approved_in_store_from_input(
            approved_input("embedded:app", "app"),
            store_limits(8),
        )
        .expect_err("manifest should fail once projected store bytes exceed the limit");

        assert!(err.to_string().contains("capacity exceeded"));
    }

    #[test]
    fn store_limits_reject_template_count_growth() {
        reset_store();

        TemplateManifestOps::replace_approved_in_store_from_input(
            approved_input("embedded:app", "app"),
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: Some(1),
                max_template_versions_per_template: None,
            },
        )
        .unwrap();

        let err = TemplateManifestOps::replace_approved_in_store_from_input(
            approved_input("embedded:scale", "scale"),
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: Some(1),
                max_template_versions_per_template: None,
            },
        )
        .expect_err("second logical template should exceed the store template limit");

        assert!(err.to_string().contains("template count exceeded"));
    }

    #[test]
    fn store_limits_reject_version_growth_per_template() {
        reset_store();

        TemplateManifestOps::replace_approved_in_store_from_input(
            approved_input_with_version("embedded:app", "app", "0.18.0"),
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: None,
                max_template_versions_per_template: Some(1),
            },
        )
        .unwrap();

        let err = TemplateManifestOps::replace_approved_in_store_from_input(
            approved_input_with_version("embedded:app", "app", "0.18.1"),
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: None,
                max_template_versions_per_template: Some(1),
            },
        )
        .expect_err("second retained version should exceed the per-template version limit");

        assert!(err.to_string().contains("version retention exceeded"));
    }

    #[test]
    fn store_status_reports_counts_and_headroom() {
        reset_store();

        TemplateManifestOps::replace_approved_in_store_from_input(
            approved_input_with_version("embedded:app", "app", "0.18.0"),
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: Some(4),
                max_template_versions_per_template: Some(3),
            },
        )
        .unwrap();
        TemplateManifestOps::replace_approved_in_store_from_input(
            approved_input_with_version("embedded:app", "app", "0.18.1"),
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: Some(4),
                max_template_versions_per_template: Some(3),
            },
        )
        .unwrap();
        TemplateManifestOps::replace_approved_in_store_from_input(
            approved_input_with_version("embedded:scale", "scale", "0.18.0"),
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: Some(4),
                max_template_versions_per_template: Some(3),
            },
        )
        .unwrap();

        let status = TemplateManifestOps::store_status_response(
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: Some(4),
                max_template_versions_per_template: Some(3),
            },
            Some(9_900),
            WasmStoreGcStatus::default(),
        );

        assert_eq!(status.gc.mode, WasmStoreGcMode::Normal);
        assert_eq!(status.gc.changed_at, 0);
        assert_eq!(status.template_count, 2);
        assert_eq!(status.release_count, 3);
        assert_eq!(status.max_templates, Some(4));
        assert_eq!(status.max_template_versions_per_template, Some(3));
        assert!(status.within_headroom);
        assert_eq!(status.templates.len(), 2);
        assert_eq!(
            status.templates[0].template_id,
            TemplateId::new("embedded:app")
        );
        assert_eq!(status.templates[0].versions, 2);
        assert_eq!(
            status.templates[1].template_id,
            TemplateId::new("embedded:scale")
        );
        assert_eq!(status.templates[1].versions, 1);
    }

    #[test]
    fn store_status_reports_gc_preparation_state() {
        reset_store();
        let status = TemplateManifestOps::store_status_response(
            store_limits(10_000),
            None,
            WasmStoreGcStatus {
                mode: WasmStoreGcMode::Prepared,
                changed_at: 77,
                prepared_at: Some(77),
                started_at: None,
                completed_at: None,
                runs_completed: 0,
            },
        );

        assert_eq!(status.gc.mode, WasmStoreGcMode::Prepared);
        assert_eq!(status.gc.changed_at, 77);
        assert_eq!(status.gc.prepared_at, Some(77));
        assert_eq!(status.gc.started_at, None);
        assert_eq!(status.gc.completed_at, None);
        assert_eq!(status.gc.runs_completed, 0);
    }
}
