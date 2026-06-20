use candid::Principal;
use canic::{
    Error,
    dto::{
        blob_storage::{BlobStorageLocalCounters, CreateCertificateResult},
        error::ErrorCode,
    },
    ids::CanisterRole,
    protocol::{
        BLOB_STORAGE_BLOBS_ARE_LIVE, BLOB_STORAGE_BLOBS_TO_DELETE,
        BLOB_STORAGE_CONFIRM_BLOB_DELETION, BLOB_STORAGE_CREATE_CERTIFICATE,
    },
};
use canic_testing_internal::pic::{
    CanicPicExt, CanicWasmBuildProfile, install_standalone_canister, upgrade_args,
};
use ic_testkit::artifacts::{read_wasm, test_target_dir, workspace_root_for};
use ic_testkit::pic::StandaloneCanisterFixture;
use std::time::Duration;

const PROBE_CRATE: &str = "blob_storage_probe";
const PROBE_ROLE: CanisterRole = CanisterRole::new("test");
const ROOT_HASH: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ROOT_HASH_BYTES: [u8; 32] = [0xaa; 32];
const SECOND_PENDING_ROOT_HASH: &str =
    "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const SECOND_PENDING_ROOT_HASH_BYTES: [u8; 32] = [0xdd; 32];
const LIVE_ONLY_ROOT_HASH: &str =
    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const LIVE_ONLY_ROOT_HASH_BYTES: [u8; 32] = [0xcc; 32];
const UNAUTHORIZED_ROOT_HASH: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const UNAUTHORIZED_ROOT_HASH_BYTES: [u8; 32] = [0xbb; 32];
const READY_TICK_LIMIT: usize = 60;
const INSTALL_CODE_RETRY_LIMIT: usize = 4;
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_mins(5);

// Verify the non-billing blob-storage gateway lifecycle through a real canister.
#[test]
fn blob_storage_gateway_lifecycle_round_trips_under_pocketic() {
    let fixture = install_standalone_canister(PROBE_CRATE, PROBE_ROLE, CanicWasmBuildProfile::Fast);
    let gateway = principal(0x67);
    let non_gateway = principal(0x90);

    assert_create_certificate_requires_controller(&fixture, non_gateway);
    assert_create_certificate_registers_live_blob(&fixture);
    assert_pending_deletion_is_gateway_filtered(&fixture, gateway, non_gateway);
    assert_stable_state_survives_upgrade(&fixture, gateway);
    assert_gateway_confirm_deletion_removes_live_blob(&fixture, gateway);
}

// Assert create-certificate remains controlled by the host-supplied guard.
fn assert_create_certificate_requires_controller(
    fixture: &StandaloneCanisterFixture,
    non_controller: Principal,
) {
    let result: Result<CreateCertificateResult, Error> = fixture.update_call_as_or_panic(
        non_controller,
        BLOB_STORAGE_CREATE_CERTIFICATE,
        (UNAUTHORIZED_ROOT_HASH.to_string(),),
    );

    let err = result.expect_err("non-controller create certificate must be denied");
    assert_eq!(err.code, ErrorCode::Unauthorized);
    assert!(!blob_is_live(fixture, UNAUTHORIZED_ROOT_HASH_BYTES));
}

// Assert create-certificate records the blob root and returns the gateway DTO.
fn assert_create_certificate_registers_live_blob(fixture: &StandaloneCanisterFixture) {
    let result: Result<CreateCertificateResult, Error> =
        fixture.update_call_or_panic(BLOB_STORAGE_CREATE_CERTIFICATE, (ROOT_HASH.to_string(),));

    let dto = result.expect("create certificate should accept canonical root hash");
    assert_eq!(dto.method, "upload");
    assert_eq!(dto.blob_hash, ROOT_HASH);

    assert_probe_counts(fixture, 1, 0, 0);

    let live = blobs_are_live(fixture);
    assert_eq!(live, vec![true, false]);
}

// Assert pending deletion roots are visible only to registered gateway callers.
fn assert_pending_deletion_is_gateway_filtered(
    fixture: &StandaloneCanisterFixture,
    gateway: Principal,
    non_gateway: Principal,
) {
    add_gateway(fixture, gateway);

    let marked: Result<bool, Error> = fixture.update_call_or_panic(
        "blob_storage_probe_mark_pending_delete",
        (ROOT_HASH.to_string(),),
    );
    assert!(marked.expect("live blob should be marked pending deletion"));
    assert_probe_counts(fixture, 1, 1, 1);

    let denied: Vec<String> =
        fixture.query_call_as_or_panic(non_gateway, BLOB_STORAGE_BLOBS_TO_DELETE, ());
    assert!(denied.is_empty());

    let allowed: Vec<String> =
        fixture.query_call_as_or_panic(gateway, BLOB_STORAGE_BLOBS_TO_DELETE, ());
    assert_eq!(allowed, vec![ROOT_HASH.to_string()]);

    create_certificate(fixture, SECOND_PENDING_ROOT_HASH);
    let marked_second: Result<bool, Error> = fixture.update_call_or_panic(
        "blob_storage_probe_mark_pending_delete",
        (SECOND_PENDING_ROOT_HASH.to_string(),),
    );
    assert!(marked_second.expect("second live blob should be marked pending deletion"));
    assert_probe_counts(fixture, 2, 2, 1);

    let pending: Vec<String> =
        fixture.query_call_as_or_panic(gateway, BLOB_STORAGE_BLOBS_TO_DELETE, ());
    assert_eq!(
        pending,
        vec![ROOT_HASH.to_string(), SECOND_PENDING_ROOT_HASH.to_string()]
    );

    let repeated_pending: Vec<String> =
        fixture.query_call_as_or_panic(gateway, BLOB_STORAGE_BLOBS_TO_DELETE, ());
    assert_eq!(repeated_pending, pending);

    assert_gateway_principal_removal_revokes_scrubber_access(fixture, gateway, &repeated_pending);
    add_gateway(fixture, gateway);
    let restored_pending: Vec<String> =
        fixture.query_call_as_or_panic(gateway, BLOB_STORAGE_BLOBS_TO_DELETE, ());
    assert_eq!(restored_pending, repeated_pending);
}

// Assert removing a gateway principal revokes pending-list and confirm access.
fn assert_gateway_principal_removal_revokes_scrubber_access(
    fixture: &StandaloneCanisterFixture,
    gateway: Principal,
    expected_pending: &[String],
) {
    let removed: Result<bool, Error> =
        fixture.update_call_or_panic("blob_storage_probe_remove_gateway", (gateway,));
    assert!(removed.expect("gateway principal should be removed"));
    assert_probe_counts(fixture, 2, 2, 0);

    let removed_again: Result<bool, Error> =
        fixture.update_call_or_panic("blob_storage_probe_remove_gateway", (gateway,));
    assert!(!removed_again.expect("repeated gateway removal should be idempotent"));

    let denied_after_removal: Vec<String> =
        fixture.query_call_as_or_panic(gateway, BLOB_STORAGE_BLOBS_TO_DELETE, ());
    assert!(denied_after_removal.is_empty());

    fixture.update_call_as_or_panic::<(), _>(
        gateway,
        BLOB_STORAGE_CONFIRM_BLOB_DELETION,
        (vec![ROOT_HASH_BYTES.to_vec()],),
    );

    add_gateway(fixture, gateway);
    assert_probe_counts(fixture, 2, 2, 1);
    let still_pending: Vec<String> =
        fixture.query_call_as_or_panic(gateway, BLOB_STORAGE_BLOBS_TO_DELETE, ());
    assert_eq!(still_pending, expected_pending);

    let removed_after_check: Result<bool, Error> =
        fixture.update_call_or_panic("blob_storage_probe_remove_gateway", (gateway,));
    assert!(removed_after_check.expect("gateway principal should be removable again"));
    assert_probe_counts(fixture, 2, 2, 0);
}

// Assert live blobs, pending deletion, and gateway principals persist across upgrade.
fn assert_stable_state_survives_upgrade(fixture: &StandaloneCanisterFixture, gateway: Principal) {
    create_certificate(fixture, LIVE_ONLY_ROOT_HASH);
    assert!(blob_is_live(fixture, LIVE_ONLY_ROOT_HASH_BYTES));
    assert_probe_counts(fixture, 3, 2, 1);

    upgrade_probe_canister(fixture);

    assert_probe_counts(fixture, 3, 2, 1);
    assert!(blob_is_live(fixture, LIVE_ONLY_ROOT_HASH_BYTES));
    assert!(!blob_is_live(fixture, ROOT_HASH_BYTES));
    assert_liveness_ordering_and_duplicates(fixture);

    let pending_after_upgrade: Vec<String> =
        fixture.query_call_as_or_panic(gateway, BLOB_STORAGE_BLOBS_TO_DELETE, ());
    assert_eq!(
        pending_after_upgrade,
        vec![ROOT_HASH.to_string(), SECOND_PENDING_ROOT_HASH.to_string()]
    );
}

// Assert gateway liveness batches preserve input order and duplicate entries.
fn assert_liveness_ordering_and_duplicates(fixture: &StandaloneCanisterFixture) {
    let live: Vec<bool> = fixture.query_call_or_panic(
        BLOB_STORAGE_BLOBS_ARE_LIVE,
        (vec![
            LIVE_ONLY_ROOT_HASH_BYTES.to_vec(),
            ROOT_HASH_BYTES.to_vec(),
            LIVE_ONLY_ROOT_HASH_BYTES.to_vec(),
            vec![0x01],
        ],),
    );

    assert_eq!(live, vec![true, false, true, false]);
}

// Assert only a registered gateway can confirm deletion for a pending blob.
fn assert_gateway_confirm_deletion_removes_live_blob(
    fixture: &StandaloneCanisterFixture,
    gateway: Principal,
) {
    fixture.update_call_as_or_panic::<(), _>(
        principal(0x91),
        BLOB_STORAGE_CONFIRM_BLOB_DELETION,
        (vec![
            ROOT_HASH_BYTES.to_vec(),
            SECOND_PENDING_ROOT_HASH_BYTES.to_vec(),
        ],),
    );

    let still_live = blobs_are_live(fixture);
    assert_eq!(still_live, vec![false, false]);

    let still_pending: Vec<String> =
        fixture.query_call_as_or_panic(gateway, BLOB_STORAGE_BLOBS_TO_DELETE, ());
    assert_eq!(
        still_pending,
        vec![ROOT_HASH.to_string(), SECOND_PENDING_ROOT_HASH.to_string()]
    );

    fixture.update_call_as_or_panic::<(), _>(
        gateway,
        BLOB_STORAGE_CONFIRM_BLOB_DELETION,
        (vec![
            vec![0x01],
            ROOT_HASH_BYTES.to_vec(),
            SECOND_PENDING_ROOT_HASH_BYTES.to_vec(),
        ],),
    );

    let live_after_gateway_confirmation = blobs_are_live(fixture);
    assert_eq!(live_after_gateway_confirmation, vec![false, false]);

    let pending_after_gateway_confirmation: Vec<String> =
        fixture.query_call_as_or_panic(gateway, BLOB_STORAGE_BLOBS_TO_DELETE, ());
    assert!(pending_after_gateway_confirmation.is_empty());
    assert_probe_counts(fixture, 1, 0, 1);

    let live_status: Result<bool, Error> =
        fixture.query_call_or_panic("blob_storage_probe_is_live", (ROOT_HASH.to_string(),));
    assert!(!live_status.expect("live status query should accept canonical root hash"));
    assert!(!blob_is_live(fixture, SECOND_PENDING_ROOT_HASH_BYTES));
    assert!(blob_is_live(fixture, LIVE_ONLY_ROOT_HASH_BYTES));
}

// Query one valid and one malformed gateway liveness entry.
fn blobs_are_live(fixture: &StandaloneCanisterFixture) -> Vec<bool> {
    fixture.query_call_or_panic(
        BLOB_STORAGE_BLOBS_ARE_LIVE,
        (vec![ROOT_HASH_BYTES.to_vec(), vec![0x01]],),
    )
}

// Query liveness for one valid gateway root.
fn blob_is_live(fixture: &StandaloneCanisterFixture, root_hash_bytes: [u8; 32]) -> bool {
    let live: Vec<bool> = fixture.query_call_or_panic(
        BLOB_STORAGE_BLOBS_ARE_LIVE,
        (vec![root_hash_bytes.to_vec()],),
    );

    live[0]
}

// Assert local blob-storage state counters exposed by the test probe.
fn assert_probe_counts(
    fixture: &StandaloneCanisterFixture,
    stored_blobs: u64,
    pending_deletions: u64,
    gateway_principals: u64,
) {
    let counts: Result<BlobStorageLocalCounters, Error> =
        fixture.query_call_or_panic("blob_storage_probe_counts", ());

    assert_eq!(
        counts.expect("probe counts query should succeed"),
        BlobStorageLocalCounters::new(stored_blobs, pending_deletions, gateway_principals)
    );
}

// Register one canonical root hash through the gateway create-certificate endpoint.
fn create_certificate(fixture: &StandaloneCanisterFixture, root_hash: &str) {
    let result: Result<CreateCertificateResult, Error> =
        fixture.update_call_or_panic(BLOB_STORAGE_CREATE_CERTIFICATE, (root_hash.to_string(),));
    result.expect("create certificate should register live blob");
}

// Register one gateway principal through the test probe helper.
fn add_gateway(fixture: &StandaloneCanisterFixture, gateway: Principal) {
    let added: Result<(), Error> =
        fixture.update_call_or_panic("blob_storage_probe_add_gateway", (gateway,));
    added.expect("gateway principal should be added");
}

// Upgrade the probe with the same compiled wasm artifact used for install.
fn upgrade_probe_canister(fixture: &StandaloneCanisterFixture) {
    fixture
        .pic()
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    let canister_id = fixture.canister_id();
    let wasm = probe_wasm();

    fixture
        .pic()
        .retry_install_code_ok(INSTALL_CODE_RETRY_LIMIT, INSTALL_CODE_COOLDOWN, || {
            fixture
                .pic()
                .upgrade_canister(canister_id, wasm.clone(), upgrade_args(), None)
                .map_err(|err| err.to_string())
        })
        .expect("probe upgrade should succeed");
    fixture
        .pic()
        .wait_for_ready(canister_id, READY_TICK_LIMIT, "blob storage post_upgrade");
}

// Read the standalone probe wasm built by `install_standalone_canister`.
fn probe_wasm() -> Vec<u8> {
    let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
    let target_dir = test_target_dir(&workspace_root, &format!("standalone-{PROBE_CRATE}"));

    read_wasm(
        &target_dir,
        PROBE_CRATE,
        CanicWasmBuildProfile::Fast.target_dir_name(),
    )
}

// Build a deterministic non-anonymous test principal from one repeated byte.
fn principal(byte: u8) -> Principal {
    Principal::self_authenticating([byte; 32])
}
