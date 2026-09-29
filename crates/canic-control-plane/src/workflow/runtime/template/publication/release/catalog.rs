use crate::workflow::runtime::template::exact_store_payload_bytes;
use crate::{
    config,
    dto::template::{TemplateManifestInput, TemplateManifestResponse},
    ids::{TemplateChunkingMode, TemplateManifestState, WasmStoreBinding},
    ops::storage::template::TemplateManifestOps,
    workflow::runtime::template::publication::{
        WasmStorePublicationWorkflow,
        cost_guard::{PUBLICATION_RECOVERY_COMMAND_KIND, PublicationCostGuard},
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

        Ok(TemplateManifestOps::approved_manifests_for_roles_response(
            &roles,
        ))
    }

    // Remove any currently approved managed release that no longer belongs to the configured fleet.
    pub fn prune_unconfigured_managed_releases() -> Result<usize, InternalError> {
        let roles = config::fleet_subnet_root_managed_release_roles()?;
        Ok(TemplateManifestOps::prune_approved_roles_not_in(&roles))
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
        let roles = config::fleet_subnet_root_managed_release_roles()?;
        if store.releases.len() != roles.len() {
            return Err(InternalError::conflict());
        }
        for role in roles {
            let mut matches = store.releases.iter().filter(|entry| entry.role == role);
            let Some(entry) = matches.next() else {
                return Err(InternalError::conflict());
            };
            if matches.next().is_some() {
                return Err(InternalError::conflict());
            }
            exact_store_payload_bytes(
                store.pid,
                &entry.template_id,
                &entry.version,
                &entry.payload_hash,
                entry.payload_size_bytes,
            )
            .await?;
            TemplateManifestOps::replace_approved_from_input(TemplateManifestInput {
                template_id: entry.template_id.clone(),
                role: entry.role.clone(),
                version: entry.version.clone(),
                payload_hash: entry.payload_hash.clone(),
                payload_size_bytes: entry.payload_size_bytes,
                store_binding: store.binding.clone(),
                chunking_mode: TemplateChunkingMode::Chunked,
                manifest_state: crate::ids::TemplateManifestState::Approved,
                approved_at: Some(IcOps::now_secs()),
                created_at: 0,
            });
        }

        Ok(())
    }
}
