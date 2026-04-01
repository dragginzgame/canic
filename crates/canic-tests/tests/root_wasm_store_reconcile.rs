// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

mod root;

use candid::encode_one;
use canic::{Error, cdk::utils::wasm::get_wasm_hash, protocol};
use canic_control_plane::{
    dto::template::{
        TemplateChunkInput, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
        TemplateManifestInput, WasmStoreAdminCommand, WasmStoreAdminResponse,
        WasmStoreOverviewResponse, WasmStoreOverviewStoreResponse, WasmStoreStatusResponse,
    },
    ids::{
        TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    },
};
use canic_internal::canister::MINIMAL;
use canic_testkit::{
    artifacts::{WasmBuildProfile, build_dfx_all, workspace_root_for},
    pic::Pic,
};
use root::harness::setup_root;
use std::{env, fs, path::PathBuf};

const CHUNK_BYTES: usize = 1024 * 1024;
const TEST_WASM_STORE_MAX_STORE_BYTES_ENV: &str = "CANIC_IMPLICIT_WASM_STORE_MAX_STORE_BYTES";
const TEST_WASM_STORE_MAX_STORE_BYTES: &str = "8388608";
const TEST_DFX_BUILD_LOCK_RELATIVE: &str = ".dfx/canic-tests-build.lock";
const CANARY_RELEASE_PAYLOAD_BYTES: usize = 1024 * 1024;
const STORE_STATUS_HEADROOM_SAFETY_BYTES: u64 = 64 * 1024;
const ROOT_WASM_STORE_ADMIN: &str = "canic_wasm_store_admin";
const ROOT_WASM_RELATIVE: &str = ".dfx/local/canisters/root/root.wasm.gz";
const UPGRADE_READY_TICK_LIMIT: usize = 120;

///
/// ReleaseFixture
///

struct ReleaseFixture {
    manifest: TemplateManifestInput,
    prepare: TemplateChunkSetPrepareInput,
    chunks: Vec<TemplateChunkInput>,
}

#[test]
fn root_post_upgrade_preserves_multi_store_current_release_binding() {
    rebuild_test_artifacts_with_small_store_limit();

    let setup = setup_root();
    let before = publication_overview(&setup.pic, setup.root_id);
    let previous_minimal = TemplateId::from("embedded:minimal".to_string());
    let previous_store = store_with_approved_template(&before, &previous_minimal);
    let target_store = alternate_publication_store(
        &setup.pic,
        setup.root_id,
        &before,
        &previous_store.binding,
        CANARY_RELEASE_PAYLOAD_BYTES,
    );
    let template_id = TemplateId::from("canary:minimal".to_string());
    let fixture = release_fixture(
        &template_id,
        "99.0.0-reconcile",
        CANARY_RELEASE_PAYLOAD_BYTES,
    );
    set_publication_store_binding(&setup.pic, setup.root_id, target_store.binding.clone());
    stage_manifest(&setup.pic, setup.root_id, &fixture.manifest);
    prepare_chunk_set(&setup.pic, setup.root_id, &fixture.prepare);

    for chunk in &fixture.chunks {
        publish_chunk(&setup.pic, setup.root_id, chunk);
    }

    publish_current_release_set_to_current_store(&setup.pic, setup.root_id);

    let published = publication_overview(&setup.pic, setup.root_id);
    let published_store = store_with_approved_template(&published, &template_id);
    assert!(
        before.stores.len() >= 2,
        "the reduced-cap bootstrap should already produce a managed multi-store fleet"
    );
    assert_eq!(
        published_store.binding, target_store.binding,
        "the current canary release should follow the selected publication binding"
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

// Rebuild the local artifacts once with a smaller implicit wasm_store ceiling for this canary.
fn rebuild_test_artifacts_with_small_store_limit() {
    // SAFETY: This integration test runs one root topology canary in isolation and
    // sets the env var immediately before spawning `dfx build --all`.
    unsafe {
        env::set_var(
            TEST_WASM_STORE_MAX_STORE_BYTES_ENV,
            TEST_WASM_STORE_MAX_STORE_BYTES,
        );
    }

    build_dfx_all(
        &workspace_root_for(env!("CARGO_MANIFEST_DIR")),
        TEST_DFX_BUILD_LOCK_RELATIVE,
        "local",
        WasmBuildProfile::Debug,
    );

    // SAFETY: The rebuild is complete, and later runtime code reads artifacts only.
    unsafe {
        env::remove_var(TEST_WASM_STORE_MAX_STORE_BYTES_ENV);
    }
}

// Query the root-owned approved-release overview for the tracked wasm_store fleet.
fn publication_overview(pic: &Pic, root_id: candid::Principal) -> WasmStoreOverviewResponse {
    let response: Result<WasmStoreOverviewResponse, Error> = pic
        .query_call(root_id, protocol::CANIC_WASM_STORE_OVERVIEW, ())
        .expect("wasm_store overview transport failed");

    response.expect("wasm_store overview application failed")
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

// Choose one alternate existing publication target with enough live headroom for the canary release.
fn alternate_publication_store<'a>(
    pic: &Pic,
    root_id: candid::Principal,
    overview: &'a WasmStoreOverviewResponse,
    excluded_binding: &WasmStoreBinding,
    payload_len: usize,
) -> &'a WasmStoreOverviewStoreResponse {
    let required_bytes = u64::try_from(payload_len).expect("payload length should fit in u64")
        + STORE_STATUS_HEADROOM_SAFETY_BYTES;

    overview
        .stores
        .iter()
        .filter(|store| &store.binding != excluded_binding)
        .filter_map(|store| {
            let status = live_store_status(pic, root_id, store.pid);
            (status.remaining_store_bytes >= required_bytes)
                .then_some((store, status.remaining_store_bytes))
        })
        .max_by_key(|(_, remaining_store_bytes)| *remaining_store_bytes)
        .map_or_else(
            || {
            panic!(
                "missing alternate wasm_store with enough headroom for a {payload_len}-byte canary release"
            )
            },
            |(store, _)| store,
        )
}

// Pin one explicit publication binding through the root admin surface.
fn set_publication_store_binding(pic: &Pic, root_id: candid::Principal, binding: WasmStoreBinding) {
    let response: Result<WasmStoreAdminResponse, Error> = pic
        .update_call(
            root_id,
            ROOT_WASM_STORE_ADMIN,
            (WasmStoreAdminCommand::SetPublicationBinding {
                binding: binding.clone(),
            },),
        )
        .expect("set publication binding transport failed");

    match response.expect("set publication binding application failed") {
        WasmStoreAdminResponse::SetPublicationBinding { binding: returned } => {
            assert_eq!(
                returned, binding,
                "root should acknowledge the exact selected publication binding"
            );
        }
        other => panic!("unexpected publication admin response: {other:?}"),
    }
}

// Build one synthetic current release fixture for the managed rollover canary.
fn release_fixture(template_id: &TemplateId, version: &str, payload_len: usize) -> ReleaseFixture {
    let version = TemplateVersion::from(version.to_string());
    let payload = vec![0xA5; payload_len];
    let payload_hash = get_wasm_hash(&payload);
    let chunk_hashes = payload
        .chunks(CHUNK_BYTES)
        .map(get_wasm_hash)
        .collect::<Vec<_>>();
    let chunks = payload
        .chunks(CHUNK_BYTES)
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
