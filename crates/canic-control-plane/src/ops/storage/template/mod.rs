mod chunked;
mod gc;

pub use chunked::TemplateChunkedOps;
pub use gc::WasmStoreGcOps;

use crate::ids::TemplateChunkKey;
use crate::{
    dto::template::{
        TemplateManifestInput, TemplateManifestResponse, WasmStoreCatalogEntryResponse,
        WasmStoreGcStatusResponse, WasmStoreOverviewStoreResponse,
        WasmStorePublicationSlotResponse, WasmStoreTemplateStatusResponse,
    },
    ids::{
        CanisterRole, TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateReleaseKey,
        TemplateVersion, WasmStoreBinding, WasmStoreGcStatus,
    },
    storage::stable::template::{TemplateManifestRecord, TemplateManifestStateStore},
};
use canic_core::__control_plane_core as cp_core;
use cp_core::{InternalError, InternalErrorOrigin, format::byte_size};
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
    #[allow(dead_code)]
    PayloadHashMismatch(TemplateReleaseKey),

    #[error("chunk set '{0}' payload size mismatch")]
    #[allow(dead_code)]
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
        Self::ops(InternalErrorOrigin::Ops, err.to_string())
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

    // Return the currently approved manifests that still belong to the configured managed release set.
    #[must_use]
    pub fn approved_manifests_for_roles_response(
        roles: &BTreeSet<CanisterRole>,
    ) -> Vec<TemplateManifestResponse> {
        Self::approved_manifests_response()
            .into_iter()
            .filter(|manifest| roles.contains(&manifest.role))
            .collect()
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

    // Return the root-owned approved-release overview for one tracked runtime wasm store.
    #[must_use]
    pub fn root_store_overview_response(
        store_binding: &WasmStoreBinding,
        store_pid: canic_core::cdk::types::Principal,
        created_at: u64,
        limits: WasmStoreLimits,
        headroom_bytes: Option<u64>,
        gc: WasmStoreGcStatus,
        publication_slot: Option<WasmStorePublicationSlotResponse>,
    ) -> WasmStoreOverviewStoreResponse {
        let manifests = TemplateManifestStateStore::export()
            .entries
            .into_iter()
            .filter(|(_, record)| {
                record.manifest_state == TemplateManifestState::Approved
                    && &record.store_binding == store_binding
            })
            .collect::<Vec<_>>();

        let approved_payload_bytes = manifests
            .iter()
            .map(|(_, record)| record.payload_size_bytes)
            .sum::<u64>();
        let remaining_approved_payload_bytes = limits
            .max_store_bytes
            .saturating_sub(approved_payload_bytes);
        let within_approved_headroom =
            headroom_bytes.is_some_and(|threshold| remaining_approved_payload_bytes <= threshold);
        let template_versions = projected_template_versions_for_manifests(&manifests);
        let approved_release_count = u32::try_from(
            template_versions
                .values()
                .map(std::collections::BTreeSet::len)
                .sum::<usize>(),
        )
        .unwrap_or(u32::MAX);
        let approved_template_count = u32::try_from(template_versions.len()).unwrap_or(u32::MAX);
        let mut approved_templates = template_versions
            .into_iter()
            .map(|(template_id, versions)| WasmStoreTemplateStatusResponse {
                template_id,
                versions: u16::try_from(versions.len()).unwrap_or(u16::MAX),
            })
            .collect::<Vec<_>>();
        approved_templates.sort_by(|left, right| left.template_id.cmp(&right.template_id));

        WasmStoreOverviewStoreResponse {
            binding: store_binding.clone(),
            pid: store_pid,
            created_at,
            publication_slot,
            gc: WasmStoreGcStatusResponse {
                mode: gc.mode,
                changed_at: gc.changed_at,
                prepared_at: gc.prepared_at,
                started_at: gc.started_at,
                completed_at: gc.completed_at,
                runs_completed: gc.runs_completed,
            },
            approved_payload_bytes,
            approved_payload_size: byte_size(approved_payload_bytes),
            max_store_bytes: limits.max_store_bytes,
            max_store_size: byte_size(limits.max_store_bytes),
            remaining_approved_payload_bytes,
            remaining_approved_payload_size: byte_size(remaining_approved_payload_bytes),
            headroom_bytes,
            headroom_size: headroom_bytes.map(byte_size),
            within_approved_headroom,
            approved_template_count,
            max_templates: limits.max_templates,
            approved_release_count,
            max_template_versions_per_template: limits.max_template_versions_per_template,
            approved_templates,
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

    // Deprecate any currently approved managed release whose role is no longer configured.
    #[must_use]
    pub fn deprecate_approved_roles_not_in(roles: &BTreeSet<CanisterRole>) -> usize {
        let mut deprecated = 0;

        for (existing_release, mut existing) in TemplateManifestStateStore::export().entries {
            if existing.manifest_state != TemplateManifestState::Approved {
                continue;
            }
            if existing.role == CanisterRole::WASM_STORE {
                continue;
            }
            if existing.chunking_mode != TemplateChunkingMode::Chunked {
                continue;
            }
            if roles.contains(&existing.role) {
                continue;
            }

            existing.manifest_state = TemplateManifestState::Deprecated;
            TemplateManifestStateStore::upsert(existing_release, existing);
            deprecated += 1;
        }

        deprecated
    }
}

// Map a manifest input DTO into the authoritative stored record.
pub(super) fn input_to_record(input: TemplateManifestInput) -> TemplateManifestRecord {
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

fn projected_template_versions_for_manifests(
    manifests: &[(TemplateReleaseKey, TemplateManifestRecord)],
) -> BTreeMap<TemplateId, BTreeSet<TemplateVersion>> {
    let mut template_versions = BTreeMap::<TemplateId, BTreeSet<TemplateVersion>>::new();

    for (release, _) in manifests {
        template_versions
            .entry(release.template_id.clone())
            .or_default()
            .insert(release.version.clone());
    }

    template_versions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::template::{TemplateChunkInput, TemplateChunkSetInput, TemplateChunkSetPrepareInput},
        ids::{TemplateChunkingMode, TemplateVersion, WasmStoreBinding, WasmStoreGcMode},
        storage::stable::template::{TemplateChunkSetStateStore, TemplateChunkStore},
    };
    use canic_core::cdk::utils::wasm::get_wasm_hash;

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

    fn approved_chunked_input(
        template_id: &'static str,
        role: &'static str,
    ) -> TemplateManifestInput {
        let mut input = approved_input(template_id, role);
        input.chunking_mode = TemplateChunkingMode::Chunked;
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
    fn deprecate_approved_roles_not_in_prunes_stale_managed_roles() {
        reset_store();

        TemplateManifestOps::replace_approved_from_input(approved_chunked_input("one", "app"));
        TemplateManifestOps::replace_approved_from_input(approved_chunked_input("two", "scale"));

        let kept = BTreeSet::from([CanisterRole::new("app")]);
        let deprecated = TemplateManifestOps::deprecate_approved_roles_not_in(&kept);

        assert_eq!(deprecated, 1);

        let approved_roles = TemplateManifestOps::approved_manifests_response()
            .into_iter()
            .map(|manifest| manifest.role)
            .collect::<Vec<_>>();

        assert_eq!(approved_roles, vec![CanisterRole::new("app")]);
    }

    #[test]
    fn publish_chunk_set_stores_info_and_chunks() {
        reset_store();

        let info = TemplateChunkedOps::publish_chunk_set_from_input(chunk_set_input(), 77).unwrap();

        assert_eq!(info.chunk_hashes.len(), 2);

        let chunk = TemplateChunkedOps::chunk_response(
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
        TemplateChunkedOps::prepare_chunk_set_from_input(
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

        let err = TemplateChunkedOps::publish_chunk_from_input(TemplateChunkInput {
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

        TemplateChunkedOps::replace_approved_in_store_from_input(
            approved_input("embedded:app", "app"),
            store_limits(10_000),
        )
        .unwrap();

        let err = TemplateChunkedOps::publish_chunk_set_in_store_from_input(
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

        let err = TemplateChunkedOps::replace_approved_in_store_from_input(
            approved_input("embedded:app", "app"),
            store_limits(8),
        )
        .expect_err("manifest should fail once projected store bytes exceed the limit");

        assert!(err.to_string().contains("capacity exceeded"));
    }

    #[test]
    fn store_limits_reject_template_count_growth() {
        reset_store();

        TemplateChunkedOps::replace_approved_in_store_from_input(
            approved_input("embedded:app", "app"),
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: Some(1),
                max_template_versions_per_template: None,
            },
        )
        .unwrap();

        let err = TemplateChunkedOps::replace_approved_in_store_from_input(
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

        TemplateChunkedOps::replace_approved_in_store_from_input(
            approved_input_with_version("embedded:app", "app", "0.18.0"),
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: None,
                max_template_versions_per_template: Some(1),
            },
        )
        .unwrap();

        let err = TemplateChunkedOps::replace_approved_in_store_from_input(
            approved_input_with_version("embedded:app", "app", "0.18.2"),
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

        TemplateChunkedOps::replace_approved_in_store_from_input(
            approved_input_with_version("embedded:app", "app", "0.18.0"),
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: Some(4),
                max_template_versions_per_template: Some(3),
            },
        )
        .unwrap();
        TemplateChunkedOps::replace_approved_in_store_from_input(
            approved_input_with_version("embedded:app", "app", "0.18.2"),
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: Some(4),
                max_template_versions_per_template: Some(3),
            },
        )
        .unwrap();
        TemplateChunkedOps::replace_approved_in_store_from_input(
            approved_input_with_version("embedded:scale", "scale", "0.18.0"),
            WasmStoreLimits {
                max_store_bytes: 10_000,
                max_templates: Some(4),
                max_template_versions_per_template: Some(3),
            },
        )
        .unwrap();

        let status = TemplateChunkedOps::store_status_response(
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
        let status = TemplateChunkedOps::store_status_response(
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
