use crate::{
    dto::template::{
        TemplateManifestResponse, WasmStoreCatalogEntryResponse, WasmStoreStatusResponse,
    },
    ids::{TemplateReleaseKey, WasmStoreBinding},
    schema::WasmStoreConfig,
    storage::stable::state::subnet::PublicationStoreStateRecord,
};
use canic_core::__control_plane_core as cp_core;
use cp_core::{InternalError, InternalErrorOrigin, cdk::types::Principal};
use std::collections::{BTreeMap, BTreeSet};

use super::WasmStorePublicationWorkflow;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PublicationStoreSnapshot {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
    pub created_at: u64,
    pub status: WasmStoreStatusResponse,
    pub releases: Vec<WasmStoreCatalogEntryResponse>,
    pub stored_chunk_hashes: Option<BTreeSet<Vec<u8>>>,
}

#[derive(Clone, Debug)]
pub(super) struct PublicationStoreFleet {
    pub preferred_binding: Option<WasmStoreBinding>,
    pub reserved_state: PublicationStoreStateRecord,
    pub stores: Vec<PublicationStoreSnapshot>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PublicationPlacementAction {
    Reuse,
    Publish,
    Create,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PublicationPlacement {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
    pub action: PublicationPlacementAction,
}

impl PublicationStoreSnapshot {
    // Return the stable release key for one catalog entry.
    fn release_key(entry: &WasmStoreCatalogEntryResponse) -> TemplateReleaseKey {
        TemplateReleaseKey::new(entry.template_id.clone(), entry.version.clone())
    }

    // Return true when this store already carries the exact release bytes for one manifest.
    pub(super) fn has_exact_release(&self, manifest: &TemplateManifestResponse) -> bool {
        self.releases.iter().any(|entry| {
            entry.role == manifest.role
                && entry.template_id == manifest.template_id
                && entry.version == manifest.version
                && entry.payload_hash == manifest.payload_hash
                && entry.payload_size_bytes == manifest.payload_size_bytes
        })
    }

    // Return any conflicting existing release occupying the same template/version key.
    pub(super) fn conflicting_release(
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
    pub(super) fn can_accept_release(&self, manifest: &TemplateManifestResponse) -> bool {
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
    pub(super) async fn ensure_stored_chunk_hashes(&mut self) -> Result<(), InternalError> {
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
    pub(super) fn record_release(&mut self, manifest: &TemplateManifestResponse) {
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

impl PublicationStoreFleet {
    // Build the writable candidate order for automatic publication decisions.
    pub(super) fn writable_store_indices(&self) -> Vec<usize> {
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
    pub(super) fn select_existing_store_for_release(
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
    pub(super) fn record_placement(
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

    pub(super) fn store_index_for_binding(&self, binding: &WasmStoreBinding) -> Option<usize> {
        self.stores
            .iter()
            .position(|store| &store.binding == binding)
    }

    // Append one newly-created empty store to the writable fleet snapshot.
    pub(super) fn push_store(
        &mut self,
        record: crate::storage::stable::state::subnet::WasmStoreRecord,
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

#[cfg(test)]
mod tests {
    use super::{PublicationPlacementAction, PublicationStoreFleet, PublicationStoreSnapshot};
    use crate::{
        dto::template::{
            TemplateManifestResponse, WasmStoreCatalogEntryResponse, WasmStoreGcStatusResponse,
            WasmStoreStatusResponse, WasmStoreTemplateStatusResponse,
        },
        ids::WasmStoreBinding,
        ids::{
            CanisterRole, TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion,
        },
        ops::storage::state::subnet::SubnetStateOps,
        storage::stable::state::subnet::{PublicationStoreStateRecord, SubnetStateRecord},
        workflow::runtime::template::publication::WasmStorePublicationWorkflow,
    };
    use candid::Principal;

    fn manifest(
        role: &'static str,
        template_id: &'static str,
        version: &'static str,
        payload_hash: u8,
        payload_size_bytes: u64,
    ) -> TemplateManifestResponse {
        TemplateManifestResponse {
            template_id: TemplateId::new(template_id),
            role: CanisterRole::new(role),
            version: TemplateVersion::new(version),
            payload_hash: vec![payload_hash; 32],
            payload_size_bytes,
            store_binding: WasmStoreBinding::new("bootstrap"),
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(10),
            created_at: 9,
        }
    }

    fn store(
        binding: &'static str,
        pid_byte: u8,
        created_at: u64,
        remaining_store_bytes: u64,
        releases: Vec<WasmStoreCatalogEntryResponse>,
        templates: Vec<WasmStoreTemplateStatusResponse>,
    ) -> PublicationStoreSnapshot {
        PublicationStoreSnapshot {
            binding: WasmStoreBinding::new(binding),
            pid: Principal::from_slice(&[pid_byte; 29]),
            created_at,
            status: WasmStoreStatusResponse {
                gc: WasmStoreGcStatusResponse {
                    mode: crate::ids::WasmStoreGcMode::Normal,
                    changed_at: 0,
                    prepared_at: None,
                    started_at: None,
                    completed_at: None,
                    runs_completed: 0,
                },
                occupied_store_bytes: 40_000_000_u64.saturating_sub(remaining_store_bytes),
                occupied_store_size: String::new(),
                max_store_bytes: 40_000_000,
                max_store_size: String::new(),
                remaining_store_bytes,
                remaining_store_size: String::new(),
                headroom_bytes: Some(4_000_000),
                headroom_size: None,
                within_headroom: remaining_store_bytes <= 4_000_000,
                template_count: u32::try_from(templates.len()).unwrap_or(u32::MAX),
                max_templates: None,
                release_count: u32::try_from(releases.len()).unwrap_or(u32::MAX),
                max_template_versions_per_template: None,
                templates,
            },
            releases,
            stored_chunk_hashes: None,
        }
    }

    #[test]
    fn promotion_is_blocked_when_it_would_overwrite_retired_binding() {
        SubnetStateOps::import(SubnetStateRecord {
            publication_store: PublicationStoreStateRecord {
                active_binding: Some(WasmStoreBinding::new("active")),
                detached_binding: Some(WasmStoreBinding::new("detached")),
                retired_binding: Some(WasmStoreBinding::new("retired")),
                generation: 3,
                changed_at: 30,
                retired_at: 20,
            },
            wasm_stores: Vec::new(),
        });

        let err =
            WasmStorePublicationWorkflow::ensure_retired_binding_slot_available_for_promotion()
                .expect_err("promotion must fail closed while retired binding is still pending");

        assert!(err.to_string().contains("rollover blocked"));
    }

    #[test]
    fn explicit_retirement_is_blocked_when_retired_binding_already_exists() {
        SubnetStateOps::import(SubnetStateRecord {
            publication_store: PublicationStoreStateRecord {
                active_binding: Some(WasmStoreBinding::new("active")),
                detached_binding: Some(WasmStoreBinding::new("detached")),
                retired_binding: Some(WasmStoreBinding::new("retired")),
                generation: 3,
                changed_at: 30,
                retired_at: 20,
            },
            wasm_stores: Vec::new(),
        });

        let err =
            WasmStorePublicationWorkflow::ensure_retired_binding_slot_available_for_retirement()
                .expect_err("retirement must fail closed while an older retired binding exists");

        assert!(err.to_string().contains("retirement blocked"));
    }

    #[test]
    fn detached_and_retired_bindings_are_not_publication_candidates() {
        let state = PublicationStoreStateRecord {
            active_binding: Some(WasmStoreBinding::new("active")),
            detached_binding: Some(WasmStoreBinding::new("detached")),
            retired_binding: Some(WasmStoreBinding::new("retired")),
            generation: 3,
            changed_at: 30,
            retired_at: 20,
        };

        assert!(
            !WasmStorePublicationWorkflow::binding_is_reserved_for_publication(
                &state,
                &WasmStoreBinding::new("active"),
            )
        );
        assert!(
            WasmStorePublicationWorkflow::binding_is_reserved_for_publication(
                &state,
                &WasmStoreBinding::new("detached"),
            )
        );
        assert!(
            WasmStorePublicationWorkflow::binding_is_reserved_for_publication(
                &state,
                &WasmStoreBinding::new("retired"),
            )
        );
    }

    #[test]
    fn exact_release_is_reused_before_new_store_is_created() {
        let manifest = manifest("app", "embedded:app", "0.20.9", 7, 512);
        let fleet = PublicationStoreFleet {
            preferred_binding: Some(WasmStoreBinding::new("primary")),
            reserved_state: PublicationStoreStateRecord::default(),
            stores: vec![store(
                "primary",
                1,
                10,
                20_000_000,
                vec![WasmStoreCatalogEntryResponse {
                    role: manifest.role.clone(),
                    template_id: manifest.template_id.clone(),
                    version: manifest.version.clone(),
                    payload_hash: manifest.payload_hash.clone(),
                    payload_size_bytes: manifest.payload_size_bytes,
                }],
                vec![WasmStoreTemplateStatusResponse {
                    template_id: manifest.template_id.clone(),
                    versions: 1,
                }],
            )],
        };

        let placement = fleet
            .select_existing_store_for_release(&manifest)
            .expect("selection must succeed")
            .expect("exact release must be reusable");

        assert_eq!(placement.binding, WasmStoreBinding::new("primary"));
        assert_eq!(placement.action, PublicationPlacementAction::Reuse);
    }

    #[test]
    fn conflicting_duplicate_release_is_rejected() {
        let manifest = manifest("app", "embedded:app", "0.20.9", 7, 512);
        let fleet = PublicationStoreFleet {
            preferred_binding: Some(WasmStoreBinding::new("primary")),
            reserved_state: PublicationStoreStateRecord::default(),
            stores: vec![store(
                "primary",
                1,
                10,
                20_000_000,
                vec![WasmStoreCatalogEntryResponse {
                    role: manifest.role.clone(),
                    template_id: manifest.template_id.clone(),
                    version: manifest.version.clone(),
                    payload_hash: vec![9; 32],
                    payload_size_bytes: manifest.payload_size_bytes,
                }],
                vec![WasmStoreTemplateStatusResponse {
                    template_id: manifest.template_id.clone(),
                    versions: 1,
                }],
            )],
        };

        let err = fleet
            .select_existing_store_for_release(&manifest)
            .expect_err("conflicting duplicate release must fail");

        assert!(err.to_string().contains("ws conflict"));
    }

    #[test]
    fn placement_uses_another_store_before_requesting_new_capacity() {
        let manifest = manifest("app", "embedded:app", "0.20.9", 7, 8_000_000);
        let fleet = PublicationStoreFleet {
            preferred_binding: Some(WasmStoreBinding::new("primary")),
            reserved_state: PublicationStoreStateRecord::default(),
            stores: vec![
                store("primary", 1, 10, 2_000_000, Vec::new(), Vec::new()),
                store("secondary", 2, 20, 16_000_000, Vec::new(), Vec::new()),
            ],
        };

        let placement = fleet
            .select_existing_store_for_release(&manifest)
            .expect("selection must succeed")
            .expect("a second store should be selected");

        assert_eq!(placement.binding, WasmStoreBinding::new("secondary"));
        assert_eq!(placement.action, PublicationPlacementAction::Publish);
    }

    #[test]
    fn reconcile_binding_ignores_older_role_versions_on_other_stores() {
        let manifest = manifest("app", "embedded:app", "0.20.10", 7, 512);
        let fleet = PublicationStoreFleet {
            preferred_binding: Some(WasmStoreBinding::new("primary")),
            reserved_state: PublicationStoreStateRecord::default(),
            stores: vec![
                store(
                    "primary",
                    1,
                    10,
                    20_000_000,
                    vec![WasmStoreCatalogEntryResponse {
                        role: manifest.role.clone(),
                        template_id: manifest.template_id.clone(),
                        version: manifest.version.clone(),
                        payload_hash: manifest.payload_hash.clone(),
                        payload_size_bytes: manifest.payload_size_bytes,
                    }],
                    vec![WasmStoreTemplateStatusResponse {
                        template_id: manifest.template_id.clone(),
                        versions: 1,
                    }],
                ),
                store(
                    "secondary",
                    2,
                    20,
                    20_000_000,
                    vec![WasmStoreCatalogEntryResponse {
                        role: manifest.role.clone(),
                        template_id: manifest.template_id.clone(),
                        version: TemplateVersion::new("0.20.9"),
                        payload_hash: vec![5; 32],
                        payload_size_bytes: manifest.payload_size_bytes,
                    }],
                    vec![WasmStoreTemplateStatusResponse {
                        template_id: manifest.template_id.clone(),
                        versions: 1,
                    }],
                ),
            ],
        };

        let binding =
            WasmStorePublicationWorkflow::reconciled_binding_for_manifest(&fleet, &manifest)
                .expect("older versions on another store must not conflict");

        assert_eq!(binding, WasmStoreBinding::new("primary"));
    }

    #[test]
    fn reconcile_binding_uses_preferred_exact_duplicate_when_current_binding_is_gone() {
        let mut manifest = manifest("app", "embedded:app", "0.20.10", 7, 512);
        manifest.store_binding = WasmStoreBinding::new("missing");

        let fleet = PublicationStoreFleet {
            preferred_binding: Some(WasmStoreBinding::new("secondary")),
            reserved_state: PublicationStoreStateRecord::default(),
            stores: vec![
                store(
                    "primary",
                    1,
                    10,
                    20_000_000,
                    vec![WasmStoreCatalogEntryResponse {
                        role: manifest.role.clone(),
                        template_id: manifest.template_id.clone(),
                        version: manifest.version.clone(),
                        payload_hash: manifest.payload_hash.clone(),
                        payload_size_bytes: manifest.payload_size_bytes,
                    }],
                    vec![WasmStoreTemplateStatusResponse {
                        template_id: manifest.template_id.clone(),
                        versions: 1,
                    }],
                ),
                store(
                    "secondary",
                    2,
                    20,
                    20_000_000,
                    vec![WasmStoreCatalogEntryResponse {
                        role: manifest.role.clone(),
                        template_id: manifest.template_id.clone(),
                        version: manifest.version.clone(),
                        payload_hash: manifest.payload_hash.clone(),
                        payload_size_bytes: manifest.payload_size_bytes,
                    }],
                    vec![WasmStoreTemplateStatusResponse {
                        template_id: manifest.template_id.clone(),
                        versions: 1,
                    }],
                ),
            ],
        };

        let binding =
            WasmStorePublicationWorkflow::reconciled_binding_for_manifest(&fleet, &manifest)
                .expect("an exact duplicate on the preferred store should be reusable");

        assert_eq!(binding, WasmStoreBinding::new("secondary"));
    }

    #[test]
    fn reconcile_binding_rejects_missing_exact_release() {
        let manifest = manifest("app", "embedded:app", "0.20.10", 7, 512);
        let fleet = PublicationStoreFleet {
            preferred_binding: Some(WasmStoreBinding::new("primary")),
            reserved_state: PublicationStoreStateRecord::default(),
            stores: vec![store(
                "primary",
                1,
                10,
                20_000_000,
                vec![WasmStoreCatalogEntryResponse {
                    role: manifest.role.clone(),
                    template_id: manifest.template_id.clone(),
                    version: TemplateVersion::new("0.20.9"),
                    payload_hash: manifest.payload_hash.clone(),
                    payload_size_bytes: manifest.payload_size_bytes,
                }],
                vec![WasmStoreTemplateStatusResponse {
                    template_id: manifest.template_id.clone(),
                    versions: 1,
                }],
            )],
        };

        let err = WasmStorePublicationWorkflow::reconciled_binding_for_manifest(&fleet, &manifest)
            .expect_err("reconcile must fail when the exact approved release disappeared");

        assert!(err.to_string().contains("missing exact release"));
    }
}
