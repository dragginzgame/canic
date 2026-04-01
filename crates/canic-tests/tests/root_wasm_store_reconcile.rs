// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

mod root;

use candid::encode_one;
use canic::{Error, cdk::utils::wasm::get_wasm_hash, protocol};
use canic_control_plane::{
    dto::template::{
        TemplateChunkInput, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
        TemplateManifestInput, WasmStoreOverviewResponse, WasmStoreOverviewStoreResponse,
        WasmStoreStatusResponse,
    },
    ids::{
        TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    },
};
use canic_internal::canister::MINIMAL;
use canic_testkit::{
    artifacts::{WasmBuildProfile, build_dfx_all_with_env, workspace_root_for},
    pic::Pic,
};
use root::harness::setup_root;
use std::{fs, path::PathBuf};

const CHUNK_BYTES: usize = 1024 * 1024;
const STORE_ROLLOVER_SAFETY_BYTES: u64 = 64 * 1024;
const TEST_DFX_BUILD_LOCK_RELATIVE: &str = ".dfx/canic-tests-reconcile-build.lock";
const TEST_SMALL_STORE_RUSTFLAGS: &str = "--cfg canic_test_small_wasm_store";
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

// Build the debug reference topology with the hidden small-cap store cfg, then install root.
fn setup_root_with_small_implicit_store() -> root::harness::RootSetup {
    let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
    build_dfx_all_with_env(
        &workspace_root,
        TEST_DFX_BUILD_LOCK_RELATIVE,
        "local",
        WasmBuildProfile::Debug,
        &[("RUSTFLAGS", TEST_SMALL_STORE_RUSTFLAGS)],
    );
    setup_root()
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
