//! Module: workflow::runtime::template::publication::snapshot
//!
//! Responsibility: model the one adopted sibling Store's live publication snapshot.
//! Does not own: Store selection, lifecycle mutation, stable state, or artifact authority.
//! Boundary: publication workflows validate and update this in-memory projection around IC calls.

use crate::{
    dto::template::{
        TemplateManifestResponse, WasmStoreCatalogEntryResponse, WasmStoreStatusResponse,
    },
    ids::{TemplateReleaseKey, WasmStoreBinding},
};
use canic_core::{
    cdk::types::Principal,
    control_plane_support::{
        error::InternalError,
        ops::{cost_guard::CostGuardPermit, ic::mgmt::MgmtOps},
    },
};
use std::collections::{BTreeMap, BTreeSet};

///
/// PublicationStoreSnapshot
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::workflow::runtime::template::publication) struct PublicationStoreSnapshot {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
    pub status: WasmStoreStatusResponse,
    pub releases: Vec<WasmStoreCatalogEntryResponse>,
    pub stored_chunk_hashes: Option<BTreeSet<Vec<u8>>>,
}

impl PublicationStoreSnapshot {
    // Return the stable release key for one catalog entry.
    fn release_key(entry: &WasmStoreCatalogEntryResponse) -> TemplateReleaseKey {
        TemplateReleaseKey::new(entry.template_id.clone(), entry.version.clone())
    }

    // Return true when this store already carries the exact release bytes for one manifest.
    pub(in crate::workflow::runtime::template::publication) fn has_exact_release(
        &self,
        manifest: &TemplateManifestResponse,
    ) -> bool {
        self.releases.iter().any(|entry| {
            entry.role == manifest.role
                && entry.template_id == manifest.template_id
                && entry.version == manifest.version
                && entry.payload_hash == manifest.payload_hash
                && entry.payload_size_bytes == manifest.payload_size_bytes
        })
    }

    // Return any conflicting existing release occupying the same template/version key.
    pub(in crate::workflow::runtime::template::publication) fn conflicting_release(
        &self,
        manifest: &TemplateManifestResponse,
    ) -> Option<&WasmStoreCatalogEntryResponse> {
        self.releases.iter().find(|entry| {
            entry.template_id == manifest.template_id
                && entry.version == manifest.version
                && (entry.role != manifest.role
                    || entry.payload_hash != manifest.payload_hash
                    || entry.payload_size_bytes != manifest.payload_size_bytes)
        })
    }

    // Return true when this store can still accept one additional release projection.
    pub(in crate::workflow::runtime::template::publication) fn can_accept_release(
        &self,
        manifest: &TemplateManifestResponse,
    ) -> bool {
        if self.has_exact_release(manifest) {
            return true;
        }

        if self.conflicting_release(manifest).is_some() {
            return false;
        }

        if self.status.remaining_store_bytes < manifest.payload_size_bytes {
            return false;
        }

        let templates = self
            .status
            .templates
            .iter()
            .map(|template| (template.template_id.clone(), template.versions))
            .collect::<BTreeMap<_, _>>();
        let current_versions = templates
            .get(&manifest.template_id)
            .copied()
            .unwrap_or_default();

        if current_versions == 0
            && self
                .status
                .max_templates
                .is_some_and(|max_templates| self.status.template_count >= max_templates)
        {
            return false;
        }

        if self
            .status
            .max_template_versions_per_template
            .is_some_and(|max_versions| current_versions >= max_versions)
        {
            return false;
        }

        true
    }

    // Load the current management-canister chunk hashes once for this store.
    pub(in crate::workflow::runtime::template::publication) async fn ensure_stored_chunk_hashes(
        &mut self,
        _publication_permit: &CostGuardPermit,
    ) -> Result<(), InternalError> {
        if self.stored_chunk_hashes.is_none() {
            self.stored_chunk_hashes = Some(
                MgmtOps::stored_chunks(self.pid)
                    .await
                    .map_err(|cause| {
                        super::error::PublicationWorkflowError::TransportUnavailable {
                            surface: "management stored_chunks",
                            cause: Box::new(cause),
                        }
                    })?
                    .into_iter()
                    .collect::<BTreeSet<_>>(),
            );
        }

        Ok(())
    }

    // Project one successful publication into the in-memory Store snapshot.
    pub(in crate::workflow::runtime::template::publication) fn record_release(
        &mut self,
        manifest: &TemplateManifestResponse,
    ) {
        if self.has_exact_release(manifest) {
            return;
        }

        self.releases.push(WasmStoreCatalogEntryResponse {
            role: manifest.role.clone(),
            template_id: manifest.template_id.clone(),
            version: manifest.version.clone(),
            payload_hash: manifest.payload_hash.clone(),
            payload_size_bytes: manifest.payload_size_bytes,
        });
        self.releases
            .sort_by(|left, right| Self::release_key(left).cmp(&Self::release_key(right)));

        self.status.occupied_store_bytes = self
            .status
            .occupied_store_bytes
            .saturating_add(manifest.payload_size_bytes);
        self.status.remaining_store_bytes = self
            .status
            .remaining_store_bytes
            .saturating_sub(manifest.payload_size_bytes);
        self.status.within_headroom = self
            .status
            .headroom_bytes
            .is_some_and(|threshold| self.status.remaining_store_bytes <= threshold);
        self.status.release_count = self.status.release_count.saturating_add(1);

        if let Some(existing) = self
            .status
            .templates
            .iter_mut()
            .find(|template| template.template_id == manifest.template_id)
        {
            existing.versions = existing.versions.saturating_add(1);
        } else {
            self.status.template_count = self.status.template_count.saturating_add(1);
            self.status
                .templates
                .push(crate::dto::template::WasmStoreTemplateStatusResponse {
                    template_id: manifest.template_id.clone(),
                    versions: 1,
                });
            self.status
                .templates
                .sort_by(|left, right| left.template_id.cmp(&right.template_id));
        }
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::template::{WasmStoreGcStatusResponse, WasmStoreStatusResponse},
        ids::{
            CanisterRole, TemplateChunkingMode, TemplateManifestState, TemplateVersion,
            WasmStoreGcMode,
        },
    };
    use canic_core::cdk::types::Principal;

    fn manifest() -> TemplateManifestResponse {
        TemplateManifestResponse {
            template_id: crate::ids::TemplateId::new("embedded:app"),
            role: CanisterRole::new("app"),
            version: TemplateVersion::new("1"),
            payload_hash: vec![1; 32],
            payload_size_bytes: 10,
            store_binding: WasmStoreBinding::new("primary"),
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(1),
            created_at: 1,
        }
    }

    fn snapshot(mode: WasmStoreGcMode) -> PublicationStoreSnapshot {
        PublicationStoreSnapshot {
            binding: WasmStoreBinding::new("primary"),
            pid: Principal::anonymous(),
            status: WasmStoreStatusResponse {
                inventory: crate::dto::template::WasmStoreInventoryResponse::default(),
                gc: WasmStoreGcStatusResponse {
                    mode,
                    changed_at: 1,
                    prepared_at: None,
                    started_at: None,
                    completed_at: None,
                    runs_completed: 0,
                },
                occupied_store_bytes: 0,
                occupied_store_size: "0 B".to_string(),
                max_store_bytes: 100,
                max_store_size: "100 B".to_string(),
                remaining_store_bytes: 100,
                remaining_store_size: "100 B".to_string(),
                headroom_bytes: None,
                headroom_size: None,
                within_headroom: false,
                template_count: 0,
                max_templates: None,
                release_count: 0,
                max_template_versions_per_template: None,
                templates: Vec::new(),
            },
            releases: Vec::new(),
            stored_chunk_hashes: None,
        }
    }

    #[test]
    fn catalog_requires_exact_release_identity_and_payload() {
        let manifest = manifest();
        let mut store = snapshot(WasmStoreGcMode::Normal);
        assert!(!store.has_exact_release(&manifest));
        store.releases.push(WasmStoreCatalogEntryResponse {
            role: manifest.role.clone(),
            template_id: manifest.template_id.clone(),
            version: manifest.version.clone(),
            payload_hash: manifest.payload_hash.clone(),
            payload_size_bytes: manifest.payload_size_bytes,
        });
        assert!(store.has_exact_release(&manifest));
        let mut changed = manifest.clone();
        changed.payload_hash[0] ^= 1;
        assert!(!store.has_exact_release(&changed));
        changed = manifest.clone();
        changed.payload_size_bytes += 1;
        assert!(!store.has_exact_release(&changed));
        changed = manifest.clone();
        changed.version = TemplateVersion::new("other");
        assert!(!store.has_exact_release(&changed));
        changed = manifest.clone();
        changed.template_id = crate::ids::TemplateId::new("other");
        assert!(!store.has_exact_release(&changed));
        changed = manifest;
        changed.role = CanisterRole::new("other");
        assert!(!store.has_exact_release(&changed));
    }
}
