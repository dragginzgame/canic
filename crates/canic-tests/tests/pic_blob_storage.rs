use candid::Principal;
use canic::{
    Error,
    dto::{
        blob_storage::{
            BlobProjectCyclesTopUpReport, BlobStorageBillingConfig, BlobStorageFundingStatus,
            BlobStorageGatewayPrincipalSyncAction, BlobStorageLocalCounters,
            BlobStoragePaymentModelStatus, BlobStorageStatusRequest, BlobStorageStatusResponse,
            CreateCertificateResult,
        },
        error::ErrorCode,
    },
    ids::CanisterRole,
    protocol::{
        BLOB_STORAGE_BLOBS_ARE_LIVE, BLOB_STORAGE_BLOBS_TO_DELETE,
        BLOB_STORAGE_CONFIRM_BLOB_DELETION, BLOB_STORAGE_CREATE_CERTIFICATE,
        BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, BLOB_STORAGE_STATUS,
        BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
    },
};
use canic_testing_internal::pic::{
    CanicPicExt, CanicWasmBuildProfile, install_standalone_canister,
    install_standalone_canister_on_pic, upgrade_args,
};
use ic_testkit::artifacts::{read_wasm, test_target_dir, workspace_root_for};
use ic_testkit::pic::{Pic, StandaloneCanisterFixture, acquire_pic_serial_guard, pic};
use std::time::Duration;

const PROBE_CRATE: &str = "blob_storage_probe";
const CASHIER_MOCK_CRATE: &str = "blob_storage_cashier_mock";
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

type MockCashierLastTopUp = Option<(Option<Principal>, Option<candid::Nat>, candid::Nat)>;

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

// Verify the 0.70 billing wrappers against a mock Cashier canister.
#[test]
fn blob_storage_billing_wrappers_round_trip_with_mock_cashier_under_pocketic() {
    let _serial_guard = acquire_pic_serial_guard();
    let pic = pic();
    let cashier_id = install_standalone_canister_on_pic(
        &pic,
        CASHIER_MOCK_CRATE,
        PROBE_ROLE,
        CanicWasmBuildProfile::Fast,
        "blob-storage-cashier-mock",
    );
    let probe_id = install_standalone_canister_on_pic(
        &pic,
        PROBE_CRATE,
        PROBE_ROLE,
        CanicWasmBuildProfile::Fast,
        "blob-storage-probe",
    );
    let gateway = principal(0x55);

    let seeded_balance: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_balance",
        (probe_id, 123_u128),
    );
    seeded_balance.expect("mock Cashier balance seed should succeed");

    let seeded_gateways: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_gateways",
        (vec![gateway, gateway],),
    );
    seeded_gateways.expect("mock Cashier gateway seed should succeed");

    let configured: Result<(), Error> = pic.update_call_or_panic(
        probe_id,
        "blob_storage_probe_configure_billing",
        (BlobStorageBillingConfig {
            cashier_canister_id: cashier_id,
            project_cycles_reserve: candid::Nat::from(1_u64),
            min_upload_balance: candid::Nat::from(10_u64),
            target_upload_balance: candid::Nat::from(100_u64),
            gateway_principal_limit: 8,
        },),
    );
    configured.expect("probe billing config should be accepted");

    let synced: Result<(), Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS, ());
    synced.expect("gateway sync should succeed");

    let counts: Result<BlobStorageLocalCounters, Error> =
        pic.query_call_or_panic(probe_id, "blob_storage_probe_counts", ());
    assert_eq!(
        counts.expect("probe counts query should succeed"),
        BlobStorageLocalCounters::new(0, 0, 1)
    );

    let balance: Result<u128, Error> = pic.update_call_or_panic(
        probe_id,
        "blob_storage_probe_cashier_total_balance",
        (cashier_id, probe_id),
    );
    assert_eq!(balance.expect("balance read should succeed"), 123);

    assert_billing_status_ready(&pic, probe_id);

    pic.add_cycles(probe_id, 10_000);
    let top_up: Result<BlobProjectCyclesTopUpReport, Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, (77_u128,));
    assert_eq!(
        top_up
            .expect("funding endpoint should reach mock Cashier")
            .attached_cycles,
        candid::Nat::from(77_u64)
    );

    let balance_after: Result<u128, Error> = pic.update_call_or_panic(
        probe_id,
        "blob_storage_probe_cashier_total_balance",
        (cashier_id, probe_id),
    );
    assert_eq!(
        balance_after.expect("balance after top-up should decode"),
        200
    );

    let last_top_up: Result<MockCashierLastTopUp, Error> =
        pic.query_call_or_panic(cashier_id, "blob_storage_cashier_mock_last_top_up", ());
    let (account, target_balance, attached_cycles) = last_top_up
        .expect("last top-up query should succeed")
        .expect("mock should record top-up");
    assert_eq!(account, Some(probe_id));
    assert_eq!(target_balance, None);
    assert_eq!(attached_cycles, candid::Nat::from(77_u64));
}

fn assert_billing_status_ready(pic: &Pic, probe_id: Principal) {
    let status: Result<BlobStorageStatusResponse, Error> = pic.update_call_or_panic(
        probe_id,
        BLOB_STORAGE_STATUS,
        (BlobStorageStatusRequest {
            sync_gateway_principals: true,
        },),
    );
    let status = status.expect("status endpoint should succeed");
    assert_eq!(
        status.payment_model,
        BlobStoragePaymentModelStatus::ProjectAsPaymentAccount
    );
    assert_eq!(
        status.gateway_principal_sync_action,
        BlobStorageGatewayPrincipalSyncAction::SkippedReadOnlyStatus
    );
    assert_eq!(status.gateway_principal_count, 1);
    assert_eq!(status.cashier_balance, Some(candid::Nat::from(123_u64)));
    assert_eq!(status.funding_status, BlobStorageFundingStatus::NotNeeded);
    assert!(status.ready);
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
