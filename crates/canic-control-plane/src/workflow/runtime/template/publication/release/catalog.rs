use crate::{
    config,
    dto::template::{TemplateManifestInput, TemplateManifestResponse},
    ids::{TemplateChunkingMode, TemplateManifestState, WasmStoreBinding},
    ops::storage::template::TemplateManifestOps,
    workflow::runtime::template::publication::{
        WasmStorePublicationWorkflow,
        cost_guard::{PUBLICATION_RECOVERY_COMMAND_KIND, PublicationCostGuard},
        fleet::{PublicationStoreFleet, PublicationStoreSnapshot},
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
        let roles = config::current_subnet_managed_release_roles()?;

        Ok(
            TemplateManifestOps::approved_manifests_for_roles_response(&roles)
                .into_iter()
                .filter(|manifest| manifest.chunking_mode == TemplateChunkingMode::Chunked)
                .collect(),
        )
    }

    // Remove any currently approved managed release that no longer belongs to the configured fleet.
    pub fn prune_unconfigured_managed_releases() -> Result<usize, InternalError> {
        let roles = config::current_subnet_managed_release_roles()?;
        Ok(TemplateManifestOps::prune_approved_roles_not_in(&roles))
    }

    // Return the exact fleet stores that already carry one approved release.
    fn exact_release_candidates<'a>(
        fleet: &'a PublicationStoreFleet,
        manifest: &TemplateManifestResponse,
    ) -> Vec<&'a PublicationStoreSnapshot> {
        let mut stores = fleet
            .stores
            .iter()
            .filter(|store| store.has_exact_release(manifest))
            .collect::<Vec<_>>();

        stores.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then(left.binding.cmp(&right.binding))
        });

        stores
    }

    // Reconcile the approved release for one role against the exact matching fleet entries.
    pub(in crate::workflow::runtime::template::publication) fn reconciled_binding_for_manifest(
        fleet: &PublicationStoreFleet,
        manifest: &TemplateManifestResponse,
    ) -> Result<WasmStoreBinding, InternalError> {
        let candidates = Self::exact_release_candidates(fleet, manifest);

        if candidates.is_empty() {
            return Err(crate::workflow::runtime::template::publication::error::PublicationWorkflowError::ExactReleaseMissing {
                role: manifest.role.clone(),
                template_id: manifest.template_id.clone(),
                version: manifest.version.clone(),
                expected_binding: manifest.store_binding.clone(),
            }
            .into());
        }

        if candidates
            .iter()
            .any(|store| store.binding == manifest.store_binding)
        {
            return Ok(manifest.store_binding.clone());
        }

        if let Some(binding) = fleet.preferred_binding.as_ref()
            && candidates.iter().any(|store| &store.binding == binding)
        {
            return Ok(binding.clone());
        }

        Ok(candidates[0].binding.clone())
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

    // Reconcile root-owned approved manifest bindings against exact releases present in the fleet.
    pub async fn import_current_store_catalog() -> Result<(), InternalError> {
        let cost_guard = PublicationCostGuard::reserve(PUBLICATION_RECOVERY_COMMAND_KIND)?;
        let result = Self::import_current_store_catalog_with_permit(cost_guard.permit()).await;
        cost_guard.settle(result)
    }

    async fn import_current_store_catalog_with_permit(
        publication_permit: &CostGuardPermit,
    ) -> Result<(), InternalError> {
        let fleet = Self::snapshot_publication_store_fleet(publication_permit).await?;
        for manifest in Self::managed_release_manifests()? {
            let binding = Self::reconciled_binding_for_manifest(&fleet, &manifest)?;
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
