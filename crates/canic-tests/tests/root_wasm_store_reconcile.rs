// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

mod root;

use candid::encode_one;
use canic::{
    CANIC_WASM_CHUNK_BYTES, Error, cdk::utils::wasm::get_wasm_hash, dto::error::ErrorCode, protocol,
};
use canic_control_plane::{
    dto::template::{
        TemplateChunkInput, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
        TemplateManifestInput, WasmStoreAdminCommand, WasmStoreAdminResponse,
        WasmStoreOverviewResponse, WasmStoreOverviewStoreResponse,
        WasmStorePublicationSlotResponse, WasmStorePublicationStatusResponse,
        WasmStorePublicationStoreStatusResponse, WasmStoreRetiredStoreStatusResponse,
        WasmStoreStatusResponse,
    },
    ids::{
        TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
        WasmStoreGcMode,
    },
};
use canic_internal::canister::MINIMAL;
use canic_testing_internal::pic::RootBaselineMetadata;
use canic_testkit::{
    artifacts::{WasmBuildProfile, workspace_root_for},
    pic::{CachedPicBaseline, Pic},
};
use root::harness::setup_root_cached_with_release_roles_profile_and_build_env;
use std::{fs, path::PathBuf, sync::Mutex};

const STORE_ROLLOVER_SAFETY_BYTES: u64 = 64 * 1024;
const TEST_SMALL_STORE_RUSTFLAGS: &str = "--cfg canic_test_small_wasm_store";
const ROOT_WASM_RELATIVE: &str = ".dfx/local/canisters/root/root.wasm.gz";
const UPGRADE_READY_TICK_LIMIT: usize = 120;
const ROOT_RECONCILE_RELEASE_ROLES: &[&str] =
    &["app", "minimal", "scale", "scale_hub", "test", "user_hub"];
const ROOT_RECONCILE_BUILD_ENV: &[(&str, &str)] = &[("RUSTFLAGS", TEST_SMALL_STORE_RUSTFLAGS)];
static ROOT_RECONCILE_BASELINE: Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> =
    Mutex::new(None);

///
/// ReleaseFixture
///

struct ReleaseFixture {
    manifest: TemplateManifestInput,
    prepare: TemplateChunkSetPrepareInput,
    chunks: Vec<TemplateChunkInput>,
}

///
/// RetiredStoreLifecycleFixture
///

struct RetiredStoreLifecycleFixture {
    before: WasmStoreOverviewResponse,
    retired_store: WasmStoreOverviewStoreResponse,
    active_store: WasmStoreOverviewStoreResponse,
    previous_generation: u64,
}

#[test]
fn root_post_upgrade_preserves_multi_store_current_release_binding() {
    let setup = setup_root_with_small_implicit_store();
    let previous_minimal = TemplateId::from("embedded:minimal".to_string());
    let before = publication_overview(&setup.pic, setup.root_id);
    let previous_store = store_with_approved_template(&before, &previous_minimal);
    let status = live_store_status(&setup.pic, setup.root_id, previous_store.pid);
    let payload_len =
        rollover_release_payload_len(status.remaining_store_bytes, status.max_store_bytes);
    let template_id = TemplateId::from("canary:minimal".to_string());
    let fixture = release_fixture(&template_id, "99.0.0-reconcile", payload_len);
    stage_manifest(&setup.pic, setup.root_id, &fixture.manifest);
    prepare_chunk_set(&setup.pic, setup.root_id, &fixture.prepare);

    for chunk in &fixture.chunks {
        publish_chunk(&setup.pic, setup.root_id, chunk);
    }

    publish_current_release_set_to_current_store(&setup.pic, setup.root_id);

    let published = publication_overview(&setup.pic, setup.root_id);
    let published_store = store_with_approved_template(&published, &template_id);
    assert!(
        published_store.binding != previous_store.binding,
        "the oversized canary release must move onto another managed wasm_store binding"
    );
    assert!(
        !has_approved_template(
            store_by_binding(&published, &previous_store.binding),
            &template_id
        ),
        "once rollover happens, the prior minimal store must not keep the approved canary release binding"
    );

    let root_wasm = fs::read(root_wasm_path()).expect("read root wasm");
    setup
        .pic
        .upgrade_canister(
            setup.root_id,
            root_wasm,
            encode_one(()).expect("encode root post_upgrade args"),
            None,
        )
        .expect("upgrade root canister");
    setup.pic.wait_for_ready(
        setup.root_id,
        UPGRADE_READY_TICK_LIMIT,
        "root post_upgrade reconcile",
    );

    let reconciled = publication_overview(&setup.pic, setup.root_id);
    let reconciled_store = store_with_approved_template(&reconciled, &template_id);
    assert_eq!(
        reconciled_store.binding, published_store.binding,
        "post_upgrade reconcile must preserve the currently approved multi-store release binding"
    );
    assert!(
        !has_approved_template(
            store_by_binding(&reconciled, &previous_store.binding),
            &template_id
        ),
        "post_upgrade must not move the current canary release back onto the original store"
    );
}

#[test]
fn root_republish_reuses_exact_release_without_allocating_another_store() {
    let setup = setup_root_with_small_implicit_store();
    let template_id = TemplateId::from("embedded:minimal".to_string());
    let before = publication_overview(&setup.pic, setup.root_id);
    let before_store = store_with_approved_template(&before, &template_id);
    let before_store_count = before.stores.len();
    let before_match_count = approved_template_store_count(&before, &template_id);

    publish_current_release_set_to_current_store(&setup.pic, setup.root_id);

    let after = publication_overview(&setup.pic, setup.root_id);
    let after_store = store_with_approved_template(&after, &template_id);
    let after_match_count = approved_template_store_count(&after, &template_id);

    assert_eq!(
        tracked_store_count(&after),
        before_store_count,
        "republishing the exact current release set must not allocate another managed wasm_store"
    );
    assert_eq!(
        after_store.binding, before_store.binding,
        "republishing an exact current release must keep the existing approved binding"
    );
    assert_eq!(
        after_match_count, before_match_count,
        "republishing an exact current release must not duplicate that approved template across stores"
    );
    assert_eq!(
        after_match_count, 1,
        "the current minimal release should remain owned by exactly one managed store"
    );
}

#[test]
fn root_conflicting_duplicate_release_is_rejected_without_fleet_mutation() {
    let setup = setup_root_with_small_implicit_store();
    let fixture = release_fixture(
        &TemplateId::from("embedded:minimal".to_string()),
        "0.21.1",
        128 * 1024,
    );
    stage_manifest(&setup.pic, setup.root_id, &fixture.manifest);
    let before = publication_overview(&setup.pic, setup.root_id);

    let err = publish_current_release_set_to_current_store_err(&setup.pic, setup.root_id)
        .expect_err("conflicting duplicate release must fail");

    assert_eq!(err.code, ErrorCode::Internal);

    let after = publication_overview(&setup.pic, setup.root_id);
    assert_eq!(
        after, before,
        "a conflicting duplicate release must not mutate the managed fleet view on failure"
    );
}

#[test]
fn root_fixed_target_conflicting_duplicate_is_rejected_without_fleet_mutation() {
    let setup = setup_root_with_small_implicit_store();
    let template_id = TemplateId::from("embedded:minimal".to_string());
    let overview = publication_overview(&setup.pic, setup.root_id);
    let target_store = store_with_approved_template(&overview, &template_id);
    let fixture = release_fixture(&template_id, "0.21.1", 128 * 1024);
    stage_manifest(&setup.pic, setup.root_id, &fixture.manifest);
    let before = publication_overview(&setup.pic, setup.root_id);

    let err = publish_current_release_set_to_store_err(&setup.pic, setup.root_id, target_store.pid)
        .expect_err("fixed-target conflicting duplicate release must fail");

    assert_eq!(err.code, ErrorCode::Internal);

    let after = publication_overview(&setup.pic, setup.root_id);
    assert_eq!(
        after, before,
        "a fixed-target conflicting duplicate release must not mutate the managed fleet view on failure"
    );
}

#[test]
fn root_fixed_target_capacity_failure_is_rejected_without_fleet_mutation() {
    let setup = setup_root_with_small_implicit_store();
    let previous_minimal = TemplateId::from("embedded:minimal".to_string());
    let current = publication_overview(&setup.pic, setup.root_id);
    let target_store = store_with_approved_template(&current, &previous_minimal);
    let status = live_store_status(&setup.pic, setup.root_id, target_store.pid);
    let payload_len =
        rollover_release_payload_len(status.remaining_store_bytes, status.max_store_bytes);
    let template_id = TemplateId::from("canary:minimal".to_string());
    let fixture = release_fixture(&template_id, "99.0.0-fixed-target", payload_len);
    stage_manifest(&setup.pic, setup.root_id, &fixture.manifest);
    prepare_chunk_set(&setup.pic, setup.root_id, &fixture.prepare);

    for chunk in &fixture.chunks {
        publish_chunk(&setup.pic, setup.root_id, chunk);
    }

    let before = publication_overview(&setup.pic, setup.root_id);

    let err = publish_current_release_set_to_store_err(&setup.pic, setup.root_id, target_store.pid)
        .expect_err("fixed-target publication should fail when the chosen store cannot fit the full managed release set");

    assert_eq!(err.code, ErrorCode::Internal);

    let after = publication_overview(&setup.pic, setup.root_id);
    assert_eq!(
        after, before,
        "a fixed-target capacity failure must not mutate the managed fleet view"
    );
}

#[test]
fn root_publication_binding_transitions_mark_active_detached_and_retired_slots() {
    let setup = setup_root_with_small_implicit_store();
    let before = publication_overview(&setup.pic, setup.root_id);
    let before_status = publication_status(&setup.pic, setup.root_id);
    let first_store = before
        .stores
        .first()
        .expect("small-store setup must produce tracked stores");
    let second_store = before
        .stores
        .iter()
        .find(|store| store.binding != first_store.binding)
        .expect("small-store setup must produce another managed store");
    let previous_generation = before.publication.generation;
    assert_publication_state(&before, None, None, None, previous_generation);
    assert_initial_publication_status(&before, &before_status);

    pin_publication_binding(&setup.pic, setup.root_id, &first_store.binding);

    let after_first_pin = publication_overview(&setup.pic, setup.root_id);
    assert_publication_state(
        &after_first_pin,
        Some(first_store.binding.clone()),
        None,
        None,
        previous_generation + 1,
    );
    assert_store_slot(
        &after_first_pin,
        &first_store.binding,
        Some(WasmStorePublicationSlotResponse::Active),
    );

    pin_publication_binding(&setup.pic, setup.root_id, &second_store.binding);

    let after_second_pin = publication_overview(&setup.pic, setup.root_id);
    assert_publication_state(
        &after_second_pin,
        Some(second_store.binding.clone()),
        Some(first_store.binding.clone()),
        None,
        previous_generation + 2,
    );
    assert_store_slot(
        &after_second_pin,
        &second_store.binding,
        Some(WasmStorePublicationSlotResponse::Active),
    );
    assert_store_slot(
        &after_second_pin,
        &first_store.binding,
        Some(WasmStorePublicationSlotResponse::Detached),
    );

    retire_detached_publication_binding(&setup.pic, setup.root_id, &first_store.binding);

    let after_retire = publication_overview(&setup.pic, setup.root_id);
    let after_retire_status = publication_status(&setup.pic, setup.root_id);
    assert_publication_state(
        &after_retire,
        Some(second_store.binding.clone()),
        None,
        Some(first_store.binding.clone()),
        previous_generation + 3,
    );
    assert_store_slot(
        &after_retire,
        &second_store.binding,
        Some(WasmStorePublicationSlotResponse::Active),
    );
    assert_store_slot(
        &after_retire,
        &first_store.binding,
        Some(WasmStorePublicationSlotResponse::Retired),
    );
    assert_retired_publication_status(
        &after_retire,
        &after_retire_status,
        &second_store.binding,
        &first_store.binding,
    );
}

#[test]
fn root_retired_store_gc_finalize_and_delete_cleans_up_tracked_store() {
    let setup = setup_root_with_small_implicit_store();
    let fixture = retire_one_publication_store(&setup.pic, setup.root_id);

    run_retired_store_gc(&setup.pic, setup.root_id, &fixture.retired_store);
    finalize_retired_store_binding(
        &setup.pic,
        setup.root_id,
        &fixture.active_store,
        &fixture.retired_store,
        fixture.previous_generation,
    );
    delete_finalized_store(
        &setup.pic,
        setup.root_id,
        &fixture.before,
        &fixture.active_store,
        &fixture.retired_store,
        fixture.previous_generation,
    );
}

// Build the debug reference topology with the hidden small-cap store cfg, then install root.
fn setup_root_with_small_implicit_store() -> root::harness::RootSetup {
    setup_root_cached_with_release_roles_profile_and_build_env(
        "cached root reconcile small-store baseline",
        &ROOT_RECONCILE_BASELINE,
        ROOT_RECONCILE_RELEASE_ROLES,
        WasmBuildProfile::Debug,
        ROOT_RECONCILE_BUILD_ENV,
    )
}

// Retire one managed store so the GC/finalize/delete canary can drive the full lifecycle.
fn retire_one_publication_store(
    pic: &Pic,
    root_id: candid::Principal,
) -> RetiredStoreLifecycleFixture {
    let before = publication_overview(pic, root_id);
    let retired_store = before
        .stores
        .first()
        .expect("small-store setup must produce tracked stores")
        .clone();
    let active_store = before
        .stores
        .iter()
        .find(|store| store.binding != retired_store.binding)
        .expect("small-store setup must produce another managed store")
        .clone();
    let previous_generation = before.publication.generation;

    let _ = admin_call(
        pic,
        root_id,
        WasmStoreAdminCommand::SetPublicationBinding {
            binding: retired_store.binding.clone(),
        },
    );
    let _ = admin_call(
        pic,
        root_id,
        WasmStoreAdminCommand::SetPublicationBinding {
            binding: active_store.binding.clone(),
        },
    );
    let retired = admin_call(pic, root_id, WasmStoreAdminCommand::RetireDetachedBinding);
    assert_eq!(
        retired,
        WasmStoreAdminResponse::RetiredDetachedBinding {
            binding: Some(retired_store.binding.clone()),
        }
    );

    let after_retire = publication_overview(pic, root_id);
    assert_publication_state(
        &after_retire,
        Some(active_store.binding.clone()),
        None,
        Some(retired_store.binding.clone()),
        previous_generation + 3,
    );
    let retired_status = retired_store_status(pic, root_id)
        .expect("retired store status must exist immediately after retirement");
    assert_eq!(retired_status.retired_binding, retired_store.binding);
    assert_eq!(retired_status.generation, previous_generation + 3);
    assert!(!retired_status.gc_ready);
    assert!(retired_status.reclaimable_store_bytes > 0);

    RetiredStoreLifecycleFixture {
        before,
        retired_store,
        active_store,
        previous_generation,
    }
}

// Drive store-local GC through prepare, begin, and complete for one retired managed store.
fn run_retired_store_gc(
    pic: &Pic,
    root_id: candid::Principal,
    retired_store: &WasmStoreOverviewStoreResponse,
) {
    let prepared = admin_call(pic, root_id, WasmStoreAdminCommand::PrepareRetiredStoreGc);
    assert_eq!(
        prepared,
        WasmStoreAdminResponse::PreparedRetiredStoreGc {
            binding: Some(retired_store.binding.clone()),
        }
    );
    let prepared_status = live_store_status(pic, root_id, retired_store.pid);
    assert_eq!(prepared_status.gc.mode, WasmStoreGcMode::Prepared);
    let prepared_retired_status =
        retired_store_status(pic, root_id).expect("retired store status must still exist");
    assert_eq!(
        prepared_retired_status.store.gc.mode,
        WasmStoreGcMode::Prepared
    );
    assert!(!prepared_retired_status.gc_ready);

    let began = admin_call(pic, root_id, WasmStoreAdminCommand::BeginRetiredStoreGc);
    assert_eq!(
        began,
        WasmStoreAdminResponse::BeganRetiredStoreGc {
            binding: Some(retired_store.binding.clone()),
        }
    );
    let in_progress_status = live_store_status(pic, root_id, retired_store.pid);
    assert_eq!(in_progress_status.gc.mode, WasmStoreGcMode::InProgress);
    let in_progress_retired_status =
        retired_store_status(pic, root_id).expect("retired store status must still exist");
    assert_eq!(
        in_progress_retired_status.store.gc.mode,
        WasmStoreGcMode::InProgress
    );
    assert!(!in_progress_retired_status.gc_ready);

    let completed = admin_call(pic, root_id, WasmStoreAdminCommand::CompleteRetiredStoreGc);
    assert_eq!(
        completed,
        WasmStoreAdminResponse::CompletedRetiredStoreGc {
            binding: Some(retired_store.binding.clone()),
        }
    );
    let completed_status = live_store_status(pic, root_id, retired_store.pid);
    assert_eq!(completed_status.gc.mode, WasmStoreGcMode::Complete);
    assert_eq!(completed_status.occupied_store_bytes, 0);
    assert_eq!(completed_status.template_count, 0);
    assert_eq!(completed_status.release_count, 0);
    let completed_retired_status =
        retired_store_status(pic, root_id).expect("retired store status must still exist");
    assert_eq!(
        completed_retired_status.store.gc.mode,
        WasmStoreGcMode::Complete
    );
    assert!(completed_retired_status.gc_ready);
    assert_eq!(completed_retired_status.reclaimable_store_bytes, 0);
}

// Finalize one retired managed store after its local GC pass has completed.
fn finalize_retired_store_binding(
    pic: &Pic,
    root_id: candid::Principal,
    active_store: &WasmStoreOverviewStoreResponse,
    retired_store: &WasmStoreOverviewStoreResponse,
    previous_generation: u64,
) {
    let finalized = admin_call(pic, root_id, WasmStoreAdminCommand::FinalizeRetiredBinding);
    assert_eq!(
        finalized,
        WasmStoreAdminResponse::FinalizedRetiredBinding {
            result: Some(
                canic_control_plane::dto::template::WasmStoreFinalizedStoreResponse {
                    binding: retired_store.binding.clone(),
                    store_pid: retired_store.pid,
                }
            ),
        }
    );

    let after_finalize = publication_overview(pic, root_id);
    assert_publication_state(
        &after_finalize,
        Some(active_store.binding.clone()),
        None,
        None,
        previous_generation + 4,
    );
    assert_store_slot(
        &after_finalize,
        &active_store.binding,
        Some(WasmStorePublicationSlotResponse::Active),
    );
    assert_store_slot(&after_finalize, &retired_store.binding, None);
    assert_eq!(
        retired_store_status(pic, root_id),
        None,
        "retired store status must disappear once the retired binding is finalized"
    );
}

// Delete one finalized managed store and assert it disappears from the tracked fleet.
fn delete_finalized_store(
    pic: &Pic,
    root_id: candid::Principal,
    before: &WasmStoreOverviewResponse,
    active_store: &WasmStoreOverviewStoreResponse,
    retired_store: &WasmStoreOverviewStoreResponse,
    previous_generation: u64,
) {
    let deleted = admin_call(
        pic,
        root_id,
        WasmStoreAdminCommand::DeleteFinalizedStore {
            binding: retired_store.binding.clone(),
            store_pid: retired_store.pid,
        },
    );
    assert_eq!(
        deleted,
        WasmStoreAdminResponse::DeletedFinalizedStore {
            binding: retired_store.binding.clone(),
            store_pid: retired_store.pid,
        }
    );

    let after_delete = publication_overview(pic, root_id);
    assert_eq!(
        tracked_store_count(&after_delete),
        tracked_store_count(before) - 1,
        "deleting one finalized retired store must remove it from the tracked fleet"
    );
    assert!(
        after_delete
            .stores
            .iter()
            .all(|store| store.binding != retired_store.binding),
        "the finalized retired store must disappear from the root-owned fleet overview after delete"
    );
    assert_publication_state(
        &after_delete,
        Some(active_store.binding.clone()),
        None,
        None,
        previous_generation + 4,
    );
    assert_eq!(
        retired_store_status(pic, root_id),
        None,
        "retired store status must stay absent after deleting the finalized store"
    );
}

// Pin one explicit publication binding and assert the typed admin result.
fn pin_publication_binding(pic: &Pic, root_id: candid::Principal, binding: &WasmStoreBinding) {
    let pinned = admin_call(
        pic,
        root_id,
        WasmStoreAdminCommand::SetPublicationBinding {
            binding: binding.clone(),
        },
    );
    assert_eq!(
        pinned,
        WasmStoreAdminResponse::SetPublicationBinding {
            binding: binding.clone(),
        }
    );
}

// Retire the current detached publication binding and assert the typed admin result.
fn retire_detached_publication_binding(
    pic: &Pic,
    root_id: candid::Principal,
    binding: &WasmStoreBinding,
) {
    let retired = admin_call(pic, root_id, WasmStoreAdminCommand::RetireDetachedBinding);
    assert_eq!(
        retired,
        WasmStoreAdminResponse::RetiredDetachedBinding {
            binding: Some(binding.clone()),
        }
    );
}

// Assert the initial live publication-status surface for the managed fleet.
fn assert_initial_publication_status(
    overview: &WasmStoreOverviewResponse,
    status: &WasmStorePublicationStatusResponse,
) {
    assert_eq!(status.publication, overview.publication);
    assert!(status.managed_release_count > 0);
    let preferred_binding = status
        .preferred_binding
        .clone()
        .expect("small-store setup must expose one preferred publication binding");
    let preferred_store = publication_status_store_by_binding(status, &preferred_binding);
    assert!(preferred_store.is_preferred_binding);
    assert!(preferred_store.is_selectable_for_publication);
    assert!(!preferred_store.is_reserved_for_publication);
    assert_eq!(preferred_store.publication_candidate_order, Some(0));
}

// Assert the live publication-status surface after one store becomes active and another retired.
fn assert_retired_publication_status(
    overview: &WasmStoreOverviewResponse,
    status: &WasmStorePublicationStatusResponse,
    active_binding: &WasmStoreBinding,
    retired_binding: &WasmStoreBinding,
) {
    assert_eq!(status.publication, overview.publication);
    assert_eq!(status.preferred_binding, Some(active_binding.clone()));

    let active_store = publication_status_store_by_binding(status, active_binding);
    assert_eq!(
        active_store.publication_slot,
        Some(WasmStorePublicationSlotResponse::Active)
    );
    assert!(active_store.is_preferred_binding);
    assert!(active_store.is_selectable_for_publication);
    assert!(!active_store.is_reserved_for_publication);
    assert_eq!(active_store.publication_candidate_order, Some(0));

    let retired_store = publication_status_store_by_binding(status, retired_binding);
    assert_eq!(
        retired_store.publication_slot,
        Some(WasmStorePublicationSlotResponse::Retired)
    );
    assert!(!retired_store.is_preferred_binding);
    assert!(!retired_store.is_selectable_for_publication);
    assert!(retired_store.is_reserved_for_publication);
    assert_eq!(retired_store.publication_candidate_order, None);
}

// Query the root-owned approved-release overview for the tracked wasm_store fleet.
fn publication_overview(pic: &Pic, root_id: candid::Principal) -> WasmStoreOverviewResponse {
    let response: Result<WasmStoreOverviewResponse, Error> = pic
        .query_call(root_id, protocol::CANIC_WASM_STORE_OVERVIEW, ())
        .expect("wasm_store overview transport failed");

    response.expect("wasm_store overview application failed")
}

// Read the live root-owned publication placement status for the current managed fleet.
fn publication_status(pic: &Pic, root_id: candid::Principal) -> WasmStorePublicationStatusResponse {
    let response: Result<WasmStorePublicationStatusResponse, Error> = pic
        .update_call(root_id, protocol::CANIC_WASM_STORE_PUBLICATION_STATUS, ())
        .expect("wasm_store publication status transport failed");

    response.expect("wasm_store publication status application failed")
}

// Read the root-owned retired-store lifecycle view for the current publication fleet.
fn retired_store_status(
    pic: &Pic,
    root_id: candid::Principal,
) -> Option<WasmStoreRetiredStoreStatusResponse> {
    let response: Result<Option<WasmStoreRetiredStoreStatusResponse>, Error> = pic
        .update_call(root_id, protocol::CANIC_WASM_STORE_RETIRED_STATUS, ())
        .expect("wasm_store retired status transport failed");

    response.expect("wasm_store retired status application failed")
}

// Return one tracked store by its logical binding.
fn store_by_binding<'a>(
    overview: &'a WasmStoreOverviewResponse,
    binding: &WasmStoreBinding,
) -> &'a WasmStoreOverviewStoreResponse {
    overview
        .stores
        .iter()
        .find(|store| &store.binding == binding)
        .unwrap_or_else(|| panic!("missing overview store for binding {binding}"))
}

// Return one tracked publication-status row by its logical binding.
fn publication_status_store_by_binding<'a>(
    status: &'a WasmStorePublicationStatusResponse,
    binding: &WasmStoreBinding,
) -> &'a WasmStorePublicationStoreStatusResponse {
    status
        .stores
        .iter()
        .find(|store| &store.binding == binding)
        .unwrap_or_else(|| panic!("missing publication status store for binding {binding}"))
}

// Return one tracked store that currently owns one approved template id.
fn store_with_approved_template<'a>(
    overview: &'a WasmStoreOverviewResponse,
    template_id: &TemplateId,
) -> &'a WasmStoreOverviewStoreResponse {
    overview
        .stores
        .iter()
        .find(|store| has_approved_template(store, template_id))
        .unwrap_or_else(|| panic!("missing approved template {template_id} in wasm_store overview"))
}

// Return true when one store currently owns an approved template id.
fn has_approved_template(store: &WasmStoreOverviewStoreResponse, template_id: &TemplateId) -> bool {
    store
        .approved_templates
        .iter()
        .any(|entry| &entry.template_id == template_id)
}

// Count how many tracked stores currently advertise one approved template id.
fn approved_template_store_count(
    overview: &WasmStoreOverviewResponse,
    template_id: &TemplateId,
) -> usize {
    overview
        .stores
        .iter()
        .filter(|store| has_approved_template(store, template_id))
        .count()
}

// Return the number of tracked runtime-managed wasm stores in the overview.
const fn tracked_store_count(overview: &WasmStoreOverviewResponse) -> usize {
    overview.stores.len()
}

// Return the current root-owned publication slot for one tracked store binding.
fn publication_slot(
    overview: &WasmStoreOverviewResponse,
    binding: &WasmStoreBinding,
) -> Option<WasmStorePublicationSlotResponse> {
    store_by_binding(overview, binding).publication_slot
}

// Assert the root-owned publication-state slots and generation.
fn assert_publication_state(
    overview: &WasmStoreOverviewResponse,
    active: Option<WasmStoreBinding>,
    detached: Option<WasmStoreBinding>,
    retired: Option<WasmStoreBinding>,
    generation: u64,
) {
    assert_eq!(overview.publication.active_binding, active);
    assert_eq!(overview.publication.detached_binding, detached);
    assert_eq!(overview.publication.retired_binding, retired);
    assert_eq!(overview.publication.generation, generation);
}

// Assert the current root-owned publication slot for one tracked store binding.
fn assert_store_slot(
    overview: &WasmStoreOverviewResponse,
    binding: &WasmStoreBinding,
    expected: Option<WasmStorePublicationSlotResponse>,
) {
    assert_eq!(publication_slot(overview, binding), expected);
}

// Stage one manifest through the root admin surface.
fn stage_manifest(pic: &Pic, root_id: candid::Principal, manifest: &TemplateManifestInput) {
    let staged: Result<(), Error> = pic
        .update_call(
            root_id,
            protocol::CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
            (manifest.clone(),),
        )
        .expect("template manifest staging transport failed");

    staged.expect("template manifest staging application failed");
}

// Prepare one chunk set through the root admin surface.
fn prepare_chunk_set(
    pic: &Pic,
    root_id: candid::Principal,
    prepare: &TemplateChunkSetPrepareInput,
) {
    let prepared: Result<TemplateChunkSetInfoResponse, Error> = pic
        .update_call(
            root_id,
            protocol::CANIC_TEMPLATE_PREPARE_ADMIN,
            (prepare.clone(),),
        )
        .expect("template prepare transport failed");

    let _ = prepared.expect("template prepare application failed");
}

// Publish one chunk through the root admin surface.
fn publish_chunk(pic: &Pic, root_id: candid::Principal, chunk: &TemplateChunkInput) {
    let published: Result<(), Error> = pic
        .update_call(
            root_id,
            protocol::CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
            (chunk.clone(),),
        )
        .expect("template chunk publish transport failed");

    published.expect("template chunk publish application failed");
}

// Publish the current approved release set through the managed store fleet.
fn publish_current_release_set_to_current_store(pic: &Pic, root_id: candid::Principal) {
    let published: Result<(), Error> = pic
        .update_call(
            root_id,
            protocol::CANIC_TEMPLATE_PUBLISH_TO_CURRENT_STORE_ADMIN,
            (),
        )
        .expect("publish current release set transport failed");

    published.expect("publish current release set application failed");
}

// Publish the current approved release set through the managed store fleet and return the typed result.
fn publish_current_release_set_to_current_store_err(
    pic: &Pic,
    root_id: candid::Principal,
) -> Result<(), Error> {
    pic.update_call(
        root_id,
        protocol::CANIC_TEMPLATE_PUBLISH_TO_CURRENT_STORE_ADMIN,
        (),
    )
    .expect("publish current release set transport failed")
}

// Publish the current managed release set into one explicit target store and return the typed result.
fn publish_current_release_set_to_store_err(
    pic: &Pic,
    root_id: candid::Principal,
    store_pid: candid::Principal,
) -> Result<WasmStoreAdminResponse, Error> {
    pic.update_call(
        root_id,
        protocol::CANIC_WASM_STORE_ADMIN,
        (WasmStoreAdminCommand::PublishCurrentReleaseToStore { store_pid },),
    )
    .expect("publish current release to store transport failed")
}

// Execute one root-owned wasm_store admin command and return the typed response.
fn admin_call(
    pic: &Pic,
    root_id: candid::Principal,
    cmd: WasmStoreAdminCommand,
) -> WasmStoreAdminResponse {
    let response: Result<WasmStoreAdminResponse, Error> = pic
        .update_call(root_id, protocol::CANIC_WASM_STORE_ADMIN, (cmd,))
        .expect("wasm_store admin transport failed");

    response.expect("wasm_store admin application failed")
}

// Query the live wasm_store canister for real occupied/remaining bytes.
fn live_store_status(
    pic: &Pic,
    root_id: candid::Principal,
    store_pid: candid::Principal,
) -> WasmStoreStatusResponse {
    let response: Result<WasmStoreStatusResponse, Error> = pic
        .query_call_as(store_pid, root_id, protocol::CANIC_WASM_STORE_STATUS, ())
        .expect("wasm_store status transport failed");

    response.expect("wasm_store status application failed")
}

// Choose one payload size that must overflow the current store but still fit an empty fresh store.
fn rollover_release_payload_len(remaining_store_bytes: u64, max_store_bytes: u64) -> usize {
    let payload_bytes = remaining_store_bytes
        .saturating_add(STORE_ROLLOVER_SAFETY_BYTES)
        .min(max_store_bytes.saturating_sub(1));

    assert!(
        payload_bytes > remaining_store_bytes,
        "the reconcile canary requires one store with a non-empty approved payload footprint"
    );

    usize::try_from(payload_bytes).expect("payload length should fit in usize")
}

// Build one synthetic current release fixture for the managed rollover canary.
fn release_fixture(template_id: &TemplateId, version: &str, payload_len: usize) -> ReleaseFixture {
    let version = TemplateVersion::from(version.to_string());
    let payload = vec![0xA5; payload_len];
    let payload_hash = get_wasm_hash(&payload);
    let chunk_hashes = payload
        .chunks(CANIC_WASM_CHUNK_BYTES)
        .map(get_wasm_hash)
        .collect::<Vec<_>>();
    let chunks = payload
        .chunks(CANIC_WASM_CHUNK_BYTES)
        .enumerate()
        .map(|(chunk_index, bytes)| TemplateChunkInput {
            template_id: template_id.clone(),
            version: version.clone(),
            chunk_index: u32::try_from(chunk_index).expect("chunk index fits in u32"),
            bytes: bytes.to_vec(),
        })
        .collect::<Vec<_>>();

    ReleaseFixture {
        manifest: TemplateManifestInput {
            template_id: template_id.clone(),
            role: MINIMAL,
            version: version.clone(),
            payload_hash: payload_hash.clone(),
            payload_size_bytes: payload_len as u64,
            store_binding: WasmStoreBinding::new("bootstrap"),
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: None,
            created_at: 0,
        },
        prepare: TemplateChunkSetPrepareInput {
            template_id: template_id.clone(),
            version,
            payload_hash,
            payload_size_bytes: payload_len as u64,
            chunk_hashes,
        },
        chunks,
    }
}

// Resolve the currently built root wasm artifact used for PocketIC upgrades.
fn root_wasm_path() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR")).join(ROOT_WASM_RELATIVE)
}
