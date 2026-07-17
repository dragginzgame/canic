use super::{PublicationPlacementAction, PublicationStoreFleet, PublicationStoreSnapshot};
use crate::{
    dto::template::{
        TemplateManifestResponse, WasmStoreCatalogEntryResponse, WasmStoreGcStatusResponse,
        WasmStoreStatusResponse, WasmStoreTemplateStatusResponse,
    },
    ids::{CanisterRole, TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion},
    ids::{WasmStoreBinding, WasmStoreGcMode},
    ops::storage::state::subnet::{
        PublicationStoreStateTestInput, SubnetStateOps, WasmStoreStateTestInput,
    },
    view::state::PublicationStoreStateView,
    workflow::runtime::template::publication::WasmStorePublicationWorkflow,
};
use candid::Principal;
use canic_core::dto::error::ErrorCode;

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

fn import_occupied_retired_slot() {
    SubnetStateOps::import_test_state(
        PublicationStoreStateTestInput {
            active_binding: Some(WasmStoreBinding::new("active")),
            detached_binding: Some(WasmStoreBinding::new("detached")),
            retired_binding: Some(WasmStoreBinding::new("retired")),
            generation: 3,
            changed_at: 30,
            retired_at: 20,
        },
        Vec::new(),
    );
}

#[test]
fn promotion_is_blocked_when_it_would_overwrite_retired_binding() {
    import_occupied_retired_slot();

    WasmStorePublicationWorkflow::ensure_retired_binding_slot_available_for_promotion()
        .expect_err("promotion must fail closed while retired binding is still pending");
}

#[test]
fn explicit_retirement_is_blocked_when_retired_binding_already_exists() {
    import_occupied_retired_slot();

    WasmStorePublicationWorkflow::ensure_retired_binding_slot_available_for_retirement()
        .expect_err("retirement must fail closed while an older retired binding exists");
}

#[test]
fn clear_binding_reports_blocked_retired_slot() {
    import_occupied_retired_slot();

    WasmStorePublicationWorkflow::clear_current_publication_store_binding()
        .expect_err("clear must fail while it would overwrite a retired slot");
    assert_eq!(
        SubnetStateOps::publication_store_state().active_binding,
        Some(WasmStoreBinding::new("active"))
    );
}

#[test]
fn detached_and_retired_bindings_are_not_publication_candidates() {
    let state = PublicationStoreStateView {
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
fn completed_gc_store_cannot_be_selected_or_reactivated() {
    let binding = WasmStoreBinding::new("finalized");
    let pid = Principal::from_slice(&[9; 29]);
    SubnetStateOps::import_test_state(
        PublicationStoreStateTestInput {
            active_binding: None,
            detached_binding: None,
            retired_binding: None,
            generation: 0,
            changed_at: 0,
            retired_at: 0,
        },
        vec![WasmStoreStateTestInput {
            binding: binding.clone(),
            pid,
            created_at: 10,
            gc_mode: WasmStoreGcMode::Complete,
            gc_changed_at: 20,
            prepared_at: Some(11),
            started_at: Some(12),
            completed_at: Some(20),
            runs_completed: 1,
        }],
    );

    let err = WasmStorePublicationWorkflow::set_current_publication_store_binding(binding.clone())
        .expect_err("completed gc store must not become active again");
    assert_eq!(
        err.public_error().map(|public| public.code),
        Some(ErrorCode::Conflict)
    );
    assert_eq!(SubnetStateOps::publication_store_binding(), None);

    let manifest = manifest("app", "embedded:app", "0.20.9", 7, 512);
    let mut finalized = store(
        "finalized",
        9,
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
    );
    finalized.status.gc.mode = WasmStoreGcMode::Complete;
    let fleet = PublicationStoreFleet {
        preferred_binding: Some(binding),
        reserved_state: PublicationStoreStateView::default(),
        stores: vec![finalized],
    };

    assert!(fleet.writable_store_indices().is_empty());
    assert!(
        fleet
            .select_existing_store_for_release(&manifest)
            .expect("selection should remain deterministic")
            .is_none()
    );
    let err = WasmStorePublicationWorkflow::reconciled_binding_for_manifest(&fleet, &manifest)
        .expect_err("finalized store must not retain release authority");
    assert_eq!(
        err.public_error().map(|public| public.code),
        Some(ErrorCode::WasmStoreManifestMissing)
    );
}

#[test]
fn exact_release_is_reused_before_new_store_is_created() {
    let manifest = manifest("app", "embedded:app", "0.20.9", 7, 512);
    let fleet = PublicationStoreFleet {
        preferred_binding: Some(WasmStoreBinding::new("primary")),
        reserved_state: PublicationStoreStateView::default(),
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
        reserved_state: PublicationStoreStateView::default(),
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

    assert_eq!(
        err.public_error().map(|public| public.code),
        Some(ErrorCode::Conflict)
    );
}

#[test]
fn placement_uses_another_store_before_requesting_new_capacity() {
    let manifest = manifest("app", "embedded:app", "0.20.9", 7, 8_000_000);
    let fleet = PublicationStoreFleet {
        preferred_binding: Some(WasmStoreBinding::new("primary")),
        reserved_state: PublicationStoreStateView::default(),
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
        reserved_state: PublicationStoreStateView::default(),
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

    let binding = WasmStorePublicationWorkflow::reconciled_binding_for_manifest(&fleet, &manifest)
        .expect("older versions on another store must not conflict");

    assert_eq!(binding, WasmStoreBinding::new("primary"));
}

#[test]
fn reconcile_binding_uses_preferred_exact_duplicate_when_current_binding_is_gone() {
    let mut manifest = manifest("app", "embedded:app", "0.20.10", 7, 512);
    manifest.store_binding = WasmStoreBinding::new("missing");

    let fleet = PublicationStoreFleet {
        preferred_binding: Some(WasmStoreBinding::new("secondary")),
        reserved_state: PublicationStoreStateView::default(),
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

    let binding = WasmStorePublicationWorkflow::reconciled_binding_for_manifest(&fleet, &manifest)
        .expect("an exact duplicate on the preferred store should be reusable");

    assert_eq!(binding, WasmStoreBinding::new("secondary"));
}

#[test]
fn reconcile_binding_rejects_missing_exact_release() {
    let manifest = manifest("app", "embedded:app", "0.20.10", 7, 512);
    let fleet = PublicationStoreFleet {
        preferred_binding: Some(WasmStoreBinding::new("primary")),
        reserved_state: PublicationStoreStateView::default(),
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

    assert_eq!(
        err.public_error().map(|public| public.code),
        Some(ErrorCode::WasmStoreManifestMissing)
    );
}
