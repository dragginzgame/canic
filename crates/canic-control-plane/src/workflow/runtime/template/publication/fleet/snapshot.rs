use crate::dto::template::{
    TemplateManifestResponse, WasmStoreCatalogEntryResponse, WasmStoreStatusResponse,
};
use crate::ids::{TemplateReleaseKey, WasmStoreBinding};
use canic_core::__control_plane_core as cp_core;
use cp_core::{InternalError, cdk::types::Principal};
use std::collections::{BTreeMap, BTreeSet};

///
/// PublicationStoreSnapshot
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::workflow::runtime::template::publication) struct PublicationStoreSnapshot {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
    pub created_at: u64,
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
    ) -> Result<(), InternalError> {
        if self.stored_chunk_hashes.is_none() {
            self.stored_chunk_hashes = Some(
                cp_core::ops::ic::mgmt::MgmtOps::stored_chunks(self.pid)
                    .await?
                    .into_iter()
                    .collect::<BTreeSet<_>>(),
            );
        }

        Ok(())
    }

    // Project one successful placement into the in-memory fleet snapshot.
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
