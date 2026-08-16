use crate::{
    config,
    dto::template::{TemplateManifestInput, TemplateManifestResponse},
    ids::{TemplateChunkingMode, TemplateManifestState, WasmStoreBinding},
    ops::storage::template::TemplateManifestOps,
    workflow::runtime::template::publication::{
        WasmStorePublicationWorkflow,
        cost_guard::{PUBLICATION_RECOVERY_COMMAND_KIND, PublicationCostGuard},
        snapshot::PublicationStoreSnapshot,
    },
};
use canic_core::control_plane_support::{
    error::InternalError,
    ops::{cost_guard::CostGuardPermit, ic::IcOps},
};

impl WasmStorePublicationWorkflow {
    // Return the deterministic approved manifests that still belong to the configured managed fleet.
    pub(in crate::workflow::runtime::template::publication) fn managed_release_manifests()
    -> Result<Vec<TemplateManifestResponse>, InternalError> {
        let roles = config::fleet_subnet_root_managed_release_roles()?;

        Ok(
            TemplateManifestOps::approved_manifests_for_roles_response(&roles)
                .into_iter()
                .filter(|manifest| manifest.chunking_mode == TemplateChunkingMode::Chunked)
                .collect(),
        )
    }

    // Remove any currently approved managed release that no longer belongs to the configured fleet.
    pub fn prune_unconfigured_managed_releases() -> Result<usize, InternalError> {
        let roles = config::fleet_subnet_root_managed_release_roles()?;
        Ok(TemplateManifestOps::prune_approved_roles_not_in(&roles))
    }

    // Reconcile one approved release against the exact adopted sibling Store.
    pub(in crate::workflow::runtime::template::publication) fn reconciled_binding_for_manifest(
        store: &PublicationStoreSnapshot,
        manifest: &TemplateManifestResponse,
    ) -> Result<WasmStoreBinding, super::super::error::PublicationWorkflowError> {
        if !store.is_available_for_publication() {
            return Err(
                super::super::error::PublicationWorkflowError::GcWriteFenced {
                    binding: store.binding.clone(),
                    mode: store.status.gc.mode,
                },
            );
        }
        if !store.has_exact_release(manifest) {
            return Err(
                super::super::error::PublicationWorkflowError::ExactReleaseMissing {
                    role: manifest.role.clone(),
                    template_id: manifest.template_id.clone(),
                    version: manifest.version.clone(),
                    expected_binding: manifest.store_binding.clone(),
                },
            );
        }
        Ok(store.binding.clone())
    }

    // Build the source label used in placement logs for one approved manifest.
    pub(super) fn release_label(manifest: &TemplateManifestResponse) -> String {
        format!("{}@{}", manifest.template_id, manifest.version)
    }

    // Mirror one approved manifest into root-owned state without mutating a live store.
    pub(super) fn mirror_manifest_to_root_state(
        _publication_permit: &CostGuardPermit,
        target_store_binding: WasmStoreBinding,
        manifest: &TemplateManifestResponse,
    ) {
        TemplateManifestOps::replace_approved_from_input(TemplateManifestInput {
            template_id: manifest.template_id.clone(),
            role: manifest.role.clone(),
            version: manifest.version.clone(),
            payload_hash: manifest.payload_hash.clone(),
            payload_size_bytes: manifest.payload_size_bytes,
            store_binding: target_store_binding,
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(IcOps::now_secs()),
            created_at: manifest.created_at,
        });
    }

    // Reconcile root-owned approved manifest bindings against the adopted Store's exact releases.
    pub async fn import_current_store_catalog() -> Result<(), InternalError> {
        let cost_guard = PublicationCostGuard::reserve(PUBLICATION_RECOVERY_COMMAND_KIND)?;
        let result = Self::import_current_store_catalog_with_permit(cost_guard.permit()).await;
        cost_guard.settle(result)
    }

    async fn import_current_store_catalog_with_permit(
        publication_permit: &CostGuardPermit,
    ) -> Result<(), InternalError> {
        let store = Self::snapshot_adopted_wasm_store(publication_permit).await?;
        Self::require_active_publication_store(&store.binding)?;
        for manifest in Self::managed_release_manifests()? {
            let binding = Self::reconciled_binding_for_manifest(&store, &manifest)?;
            TemplateManifestOps::replace_approved_from_input(TemplateManifestInput {
                template_id: manifest.template_id,
                role: manifest.role,
                version: manifest.version,
                payload_hash: manifest.payload_hash,
                payload_size_bytes: manifest.payload_size_bytes,
                store_binding: binding,
                chunking_mode: manifest.chunking_mode,
                manifest_state: manifest.manifest_state,
                approved_at: manifest.approved_at,
                created_at: manifest.created_at,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::template::{WasmStoreGcStatusResponse, WasmStoreStatusResponse},
        ids::{
            CanisterRole, TemplateChunkingMode, TemplateManifestState, TemplateVersion,
            WasmStoreGcMode,
        },
        workflow::runtime::template::publication::error::PublicationWorkflowError,
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
    fn catalog_reconciliation_distinguishes_gc_fence_from_missing_release() {
        let manifest = manifest();
        assert!(matches!(
            WasmStorePublicationWorkflow::reconciled_binding_for_manifest(
                &snapshot(WasmStoreGcMode::Prepared),
                &manifest,
            ),
            Err(PublicationWorkflowError::GcWriteFenced {
                mode: WasmStoreGcMode::Prepared,
                ..
            })
        ));
        assert!(matches!(
            WasmStorePublicationWorkflow::reconciled_binding_for_manifest(
                &snapshot(WasmStoreGcMode::Normal),
                &manifest,
            ),
            Err(PublicationWorkflowError::ExactReleaseMissing { .. })
        ));
    }
}
