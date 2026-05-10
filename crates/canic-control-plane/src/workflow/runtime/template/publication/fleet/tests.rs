use super::{PublicationPlacementAction, PublicationStoreFleet, PublicationStoreSnapshot};
use crate::{
    dto::template::{
        TemplateManifestResponse, WasmStoreCatalogEntryResponse, WasmStoreGcStatusResponse,
        WasmStoreStatusResponse, WasmStoreTemplateStatusResponse,
    },
    ids::WasmStoreBinding,
    ids::{CanisterRole, TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion},
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

    let err = WasmStorePublicationWorkflow::ensure_retired_binding_slot_available_for_promotion()
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

    let err = WasmStorePublicationWorkflow::ensure_retired_binding_slot_available_for_retirement()
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

    let binding = WasmStorePublicationWorkflow::reconciled_binding_for_manifest(&fleet, &manifest)
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
