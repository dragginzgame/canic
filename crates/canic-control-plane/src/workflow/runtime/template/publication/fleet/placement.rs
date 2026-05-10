use super::PublicationStoreSnapshot;
use crate::{
    dto::template::{TemplateManifestResponse, WasmStoreStatusResponse},
    ids::WasmStoreBinding,
    schema::WasmStoreConfig,
    storage::stable::state::subnet::{PublicationStoreStateRecord, WasmStoreRecord},
    workflow::runtime::template::publication::WasmStorePublicationWorkflow,
};
use canic_core::__control_plane_core as cp_core;
use cp_core::{InternalError, InternalErrorOrigin, cdk::types::Principal};
use std::collections::BTreeSet;

///
/// PublicationStoreFleet
///

#[derive(Clone, Debug)]
pub(in crate::workflow::runtime::template::publication) struct PublicationStoreFleet {
    pub preferred_binding: Option<WasmStoreBinding>,
    pub reserved_state: PublicationStoreStateRecord,
    pub stores: Vec<PublicationStoreSnapshot>,
}

///
/// PublicationPlacementAction
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::workflow::runtime::template::publication) enum PublicationPlacementAction {
    Reuse,
    Publish,
    Create,
}

///
/// PublicationPlacement
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::workflow::runtime::template::publication) struct PublicationPlacement {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
    pub action: PublicationPlacementAction,
}

impl PublicationStoreFleet {
    // Build the writable candidate order for automatic publication decisions.
    pub(in crate::workflow::runtime::template::publication) fn writable_store_indices(
        &self,
    ) -> Vec<usize> {
        let mut indexed = self
            .stores
            .iter()
            .enumerate()
            .filter(|(_, store)| {
                !WasmStorePublicationWorkflow::binding_is_reserved_for_publication(
                    &self.reserved_state,
                    &store.binding,
                )
            })
            .collect::<Vec<_>>();

        indexed.sort_by(|(_, left), (_, right)| {
            let left_rank = usize::from(self.preferred_binding.as_ref() != Some(&left.binding));
            let right_rank = usize::from(self.preferred_binding.as_ref() != Some(&right.binding));

            left_rank
                .cmp(&right_rank)
                .then(left.created_at.cmp(&right.created_at))
                .then(left.binding.cmp(&right.binding))
        });

        indexed.into_iter().map(|(index, _)| index).collect()
    }

    // Resolve one exact reusable placement or one publishable writable store.
    pub(in crate::workflow::runtime::template::publication) fn select_existing_store_for_release(
        &self,
        manifest: &TemplateManifestResponse,
    ) -> Result<Option<PublicationPlacement>, InternalError> {
        let mut exact_match = None;

        for index in self.writable_store_indices() {
            let store = &self.stores[index];

            if let Some(conflict) = store.conflicting_release(manifest) {
                return Err(InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!(
                        "ws conflict for {}@{} on {}: existing hash/size differ ({:?}, {})",
                        manifest.template_id,
                        manifest.version,
                        store.binding,
                        conflict.payload_hash,
                        conflict.payload_size_bytes
                    ),
                ));
            }

            if store.has_exact_release(manifest) {
                exact_match = Some(PublicationPlacement {
                    binding: store.binding.clone(),
                    pid: store.pid,
                    action: PublicationPlacementAction::Reuse,
                });
                break;
            }
        }

        if exact_match.is_some() {
            return Ok(exact_match);
        }

        for index in self.writable_store_indices() {
            let store = &self.stores[index];

            if store.can_accept_release(manifest) {
                return Ok(Some(PublicationPlacement {
                    binding: store.binding.clone(),
                    pid: store.pid,
                    action: PublicationPlacementAction::Publish,
                }));
            }
        }

        Ok(None)
    }

    // Project one successful placement back into the fleet snapshot.
    pub(in crate::workflow::runtime::template::publication) fn record_placement(
        &mut self,
        binding: &WasmStoreBinding,
        manifest: &TemplateManifestResponse,
    ) {
        if let Some(store) = self
            .stores
            .iter_mut()
            .find(|store| &store.binding == binding)
        {
            store.record_release(manifest);
        }
    }

    pub(in crate::workflow::runtime::template::publication) fn store_index_for_binding(
        &self,
        binding: &WasmStoreBinding,
    ) -> Option<usize> {
        self.stores
            .iter()
            .position(|store| &store.binding == binding)
    }

    // Append one newly-created empty store to the writable fleet snapshot.
    pub(in crate::workflow::runtime::template::publication) fn push_store(
        &mut self,
        record: WasmStoreRecord,
        config: WasmStoreConfig,
    ) {
        self.stores.push(PublicationStoreSnapshot {
            binding: record.binding,
            pid: record.pid,
            created_at: record.created_at,
            status: WasmStoreStatusResponse {
                gc: crate::dto::template::WasmStoreGcStatusResponse {
                    mode: record.gc.mode,
                    changed_at: record.gc.changed_at,
                    prepared_at: record.gc.prepared_at,
                    started_at: record.gc.started_at,
                    completed_at: record.gc.completed_at,
                    runs_completed: record.gc.runs_completed,
                },
                occupied_store_bytes: 0,
                occupied_store_size: "0.00 B".to_string(),
                max_store_bytes: config.max_store_bytes(),
                max_store_size: cp_core::format::byte_size(config.max_store_bytes()),
                remaining_store_bytes: config.max_store_bytes(),
                remaining_store_size: cp_core::format::byte_size(config.max_store_bytes()),
                headroom_bytes: config.headroom_bytes(),
                headroom_size: config.headroom_bytes().map(cp_core::format::byte_size),
                within_headroom: false,
                template_count: 0,
                max_templates: config.max_templates(),
                release_count: 0,
                max_template_versions_per_template: config.max_template_versions_per_template(),
                templates: Vec::new(),
            },
            releases: Vec::new(),
            stored_chunk_hashes: Some(BTreeSet::new()),
        });
    }
}
