use candid::Principal;
use canic::{
    Error,
    dto::{
        blob_storage::{
            BlobProjectCyclesTopUpReport, BlobStorageBillingConfig, BlobStorageBillingWarning,
            BlobStorageCashierAccountBalanceGetError, BlobStorageCashierAccountTopUpError,
            BlobStorageFundingStatus, BlobStorageGatewayPrincipalSyncAction,
            BlobStorageLocalCounters, BlobStoragePaymentModelStatus, BlobStorageReadinessBlocker,
            BlobStorageStatusRequest, BlobStorageStatusResponse, CreateCertificateResult,
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
    let _serial_guard = acquire_pic_serial_guard();
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
    let (cashier_id, probe_id) = install_billing_canisters(&pic);
    let gateway = principal(0x55);

    assert_billing_endpoints_require_controller(&pic, probe_id);
    assert_mock_failure_controls_require_controller(&pic, cashier_id);
    seed_mock_cashier_for_billing_flow(&pic, cashier_id, probe_id, gateway);
    configure_billing(&pic, cashier_id, probe_id);
    assert_initial_gateway_sync_succeeds(&pic, probe_id);

    assert_gateway_sync_rejects_invalid_cashier_list_without_mutation(&pic, cashier_id, probe_id);

    let balance: Result<u128, Error> = pic.update_call_or_panic(
        probe_id,
        "blob_storage_probe_cashier_total_balance",
        (cashier_id, probe_id),
    );
    assert_eq!(balance.expect("balance read should succeed"), 123);

    assert_billing_status_ready(&pic, probe_id);
    assert_billing_status_reports_cashier_balance_unavailable(&pic, cashier_id, probe_id);
    assert_billing_status_ready(&pic, probe_id);
    assert_billing_status_reports_cashier_balance_malformed(&pic, cashier_id, probe_id);
    assert_billing_status_ready(&pic, probe_id);

    let zero_top_up: Result<BlobProjectCyclesTopUpReport, Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, (0_u128,));
    assert_eq!(
        zero_top_up
            .expect_err("zero-cycle funding should be rejected")
            .code,
        ErrorCode::InvalidInput
    );

    pic.add_cycles(probe_id, 10_000);
    assert_cashier_top_up_error_maps_to_public_code(
        &pic,
        cashier_id,
        probe_id,
        BlobStorageCashierAccountTopUpError::NotAuthorized(probe_id),
        ErrorCode::Forbidden,
    );
    assert_cashier_top_up_error_maps_to_public_code(
        &pic,
        cashier_id,
        probe_id,
        BlobStorageCashierAccountTopUpError::AccountBalanceOverflow,
        ErrorCode::ResourceExhausted,
    );
    assert_cashier_top_up_error_maps_to_public_code(
        &pic,
        cashier_id,
        probe_id,
        BlobStorageCashierAccountTopUpError::InternalError("mock failure".to_string()),
        ErrorCode::Internal,
    );
    assert_cashier_top_up_error_maps_to_public_code(
        &pic,
        cashier_id,
        probe_id,
        BlobStorageCashierAccountTopUpError::TopUpWithoutCycles,
        ErrorCode::InvalidInput,
    );
    assert_cashier_top_up_malformed_balance_maps_to_rpc_malformed(&pic, cashier_id, probe_id);
    assert_reserve_violation_does_not_partially_top_up(&pic, cashier_id, probe_id);

    let top_up: Result<BlobProjectCyclesTopUpReport, Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, (77_u128,));
    let report = top_up.expect("funding endpoint should reach mock Cashier");
    assert_successful_funding_report(&report, 77, 1, 200);

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

// Verify status reports endpoint-visible billing readiness blockers.
#[test]
fn blob_storage_billing_status_matrix_reports_readiness_blockers_under_pocketic() {
    let _serial_guard = acquire_pic_serial_guard();
    let pic = pic();
    let (cashier_id, probe_id) = install_billing_canisters(&pic);
    let gateway = principal(0x59);

    seed_mock_cashier_for_billing_flow(&pic, cashier_id, probe_id, gateway);
    configure_billing(&pic, cashier_id, probe_id);
    assert_billing_status_reports_missing_gateways(&pic, probe_id);

    assert_initial_gateway_sync_succeeds(&pic, probe_id);
    set_mock_cashier_balance(&pic, cashier_id, probe_id, 9);
    assert_billing_status_reports_funding_required(&pic, probe_id);

    let project_cycles_available =
        status_project_cycles_available(&billing_status(&pic, probe_id, false));
    assert!(
        project_cycles_available > 1,
        "probe should have cycles available for reserve-status coverage"
    );
    configure_billing_with_reserve(&pic, cashier_id, probe_id, project_cycles_available - 1);
    assert_billing_status_reports_reserve_violation(&pic, probe_id);
}

// Verify billing config, synced gateways, and sync metadata persist across upgrade.
#[test]
fn blob_storage_billing_state_survives_upgrade_under_pocketic() {
    let _serial_guard = acquire_pic_serial_guard();
    let pic = pic();
    let (cashier_id, probe_id) = install_billing_canisters(&pic);
    let gateway = principal(0x56);

    seed_mock_cashier_for_billing_flow(&pic, cashier_id, probe_id, gateway);
    configure_billing(&pic, cashier_id, probe_id);
    assert_initial_gateway_sync_succeeds(&pic, probe_id);
    create_certificate_on_pic(&pic, probe_id, ROOT_HASH);
    mark_pending_delete_on_pic(&pic, probe_id, ROOT_HASH);
    assert_gateway_pending_roots_on_pic(&pic, probe_id, gateway, &[ROOT_HASH]);

    let status_before = billing_status(&pic, probe_id, false);
    let sync_at_before =
        assert_billing_status_matches_config(&status_before, cashier_id, probe_id, 1, 1);

    upgrade_probe_canister_on_pic(&pic, probe_id);

    let status_after = billing_status(&pic, probe_id, false);
    assert_eq!(
        assert_billing_status_matches_config(&status_after, cashier_id, probe_id, 1, 1),
        sync_at_before,
        "last successful gateway sync timestamp must survive upgrade"
    );
    assert_gateway_pending_roots_on_pic(&pic, probe_id, gateway, &[ROOT_HASH]);
    assert_status_request_does_not_sync_gateways_after_upgrade(
        &pic,
        cashier_id,
        probe_id,
        gateway,
        sync_at_before,
    );
    assert_explicit_gateway_sync_works_after_upgrade(
        &pic,
        cashier_id,
        probe_id,
        gateway,
        sync_at_before,
    );

    pic.add_cycles(probe_id, 10_000);
    let top_up: Result<BlobProjectCyclesTopUpReport, Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, (22_u128,));
    assert_eq!(
        top_up
            .expect("post-upgrade funding endpoint should not inherit a stale transient lock")
            .attached_cycles,
        candid::Nat::from(22_u64)
    );
}

// Verify missing billing config stays explicit and read-only across upgrade.
#[test]
fn blob_storage_missing_billing_config_status_survives_upgrade_under_pocketic() {
    let _serial_guard = acquire_pic_serial_guard();
    let pic = pic();
    let probe_id = install_probe_canister(&pic);
    let gateway = principal(0x58);

    add_gateway_on_pic(&pic, probe_id, gateway);
    create_certificate_on_pic(&pic, probe_id, ROOT_HASH);
    mark_pending_delete_on_pic(&pic, probe_id, ROOT_HASH);
    assert_gateway_pending_roots_on_pic(&pic, probe_id, gateway, &[ROOT_HASH]);
    assert_missing_billing_config_status(&billing_status(&pic, probe_id, true), 1);

    upgrade_probe_canister_on_pic(&pic, probe_id);

    assert_missing_billing_config_status(&billing_status(&pic, probe_id, true), 1);
    assert_gateway_pending_roots_on_pic(&pic, probe_id, gateway, &[ROOT_HASH]);

    let top_up: Result<BlobProjectCyclesTopUpReport, Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, (22_u128,));
    assert_eq!(
        top_up
            .expect_err("missing billing config should still block funding after upgrade")
            .code,
        ErrorCode::InvalidInput
    );
}

fn seed_mock_cashier_for_billing_flow(
    pic: &Pic,
    cashier_id: Principal,
    probe_id: Principal,
    gateway: Principal,
) {
    set_mock_cashier_balance(pic, cashier_id, probe_id, 123);

    let seeded_gateways: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_gateways",
        (vec![gateway, gateway],),
    );
    seeded_gateways.expect("mock Cashier gateway seed should succeed");
}

fn set_mock_cashier_balance(pic: &Pic, cashier_id: Principal, account: Principal, balance: u128) {
    let seeded_balance: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_balance",
        (account, balance),
    );
    seeded_balance.expect("mock Cashier balance seed should succeed");
}

fn assert_billing_endpoints_require_controller(pic: &Pic, probe_id: Principal) {
    let non_controller = principal(0x92);

    let sync_denied: Result<(), Error> = pic.update_call_as_or_panic(
        probe_id,
        non_controller,
        BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
        (),
    );
    assert_eq!(
        sync_denied
            .expect_err("non-controller must not sync billing gateways")
            .code,
        ErrorCode::Unauthorized
    );

    let fund_denied: Result<BlobProjectCyclesTopUpReport, Error> = pic.update_call_as_or_panic(
        probe_id,
        non_controller,
        BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
        (11_u128,),
    );
    assert_eq!(
        fund_denied
            .expect_err("non-controller must not fund blob-storage billing")
            .code,
        ErrorCode::Unauthorized
    );

    let status_denied: Result<BlobStorageStatusResponse, Error> = pic.update_call_as_or_panic(
        probe_id,
        non_controller,
        BLOB_STORAGE_STATUS,
        (BlobStorageStatusRequest {
            sync_gateway_principals: true,
        },),
    );
    assert_eq!(
        status_denied
            .expect_err("non-controller must not read guarded billing status")
            .code,
        ErrorCode::Unauthorized
    );
}

fn add_gateway_on_pic(pic: &Pic, probe_id: Principal, gateway: Principal) {
    let added: Result<(), Error> =
        pic.update_call_or_panic(probe_id, "blob_storage_probe_add_gateway", (gateway,));
    added.expect("gateway principal should be added");
}

fn assert_status_request_does_not_sync_gateways_after_upgrade(
    pic: &Pic,
    cashier_id: Principal,
    probe_id: Principal,
    original_gateway: Principal,
    sync_at_before: u64,
) {
    let replacement_gateway = principal(0x57);
    let seeded_gateways: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_gateways",
        (vec![replacement_gateway],),
    );
    seeded_gateways.expect("mock Cashier replacement gateway seed should succeed");

    let status = billing_status(pic, probe_id, true);
    assert_eq!(
        status.gateway_principal_sync_action,
        BlobStorageGatewayPrincipalSyncAction::SkippedReadOnlyStatus
    );
    assert_eq!(status.gateway_principal_count, 1);
    assert_eq!(
        status.last_gateway_principal_sync_at_ns,
        Some(sync_at_before),
        "read-only status must not record a new gateway sync timestamp"
    );
    assert!(
        status
            .warnings
            .contains(&BlobStorageBillingWarning::SyncRequestedButStatusIsReadOnly)
    );
    assert_gateway_pending_roots_on_pic(pic, probe_id, original_gateway, &[ROOT_HASH]);
    assert_gateway_pending_roots_on_pic(pic, probe_id, replacement_gateway, &[]);
}

fn assert_explicit_gateway_sync_works_after_upgrade(
    pic: &Pic,
    cashier_id: Principal,
    probe_id: Principal,
    original_gateway: Principal,
    sync_at_before: u64,
) {
    let replacement_gateway = principal(0x57);

    pic.advance_time(Duration::from_nanos(1));
    pic.tick();

    let synced: Result<(), Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS, ());
    synced.expect("explicit gateway sync should still work after upgrade");

    let status = billing_status(pic, probe_id, false);
    let sync_at_after = assert_billing_status_matches_config(&status, cashier_id, probe_id, 1, 1);
    assert!(
        sync_at_after > sync_at_before,
        "explicit sync should record a fresh post-upgrade gateway-sync timestamp"
    );
    assert_gateway_pending_roots_on_pic(pic, probe_id, original_gateway, &[]);
    assert_gateway_pending_roots_on_pic(pic, probe_id, replacement_gateway, &[ROOT_HASH]);
}

fn create_certificate_on_pic(pic: &Pic, probe_id: Principal, root_hash: &str) {
    let result: Result<CreateCertificateResult, Error> = pic.update_call_or_panic(
        probe_id,
        BLOB_STORAGE_CREATE_CERTIFICATE,
        (root_hash.to_string(),),
    );
    result.expect("create certificate should register live blob");
}

fn mark_pending_delete_on_pic(pic: &Pic, probe_id: Principal, root_hash: &str) {
    let marked: Result<bool, Error> = pic.update_call_or_panic(
        probe_id,
        "blob_storage_probe_mark_pending_delete",
        (root_hash.to_string(),),
    );
    assert!(marked.expect("live blob should be marked pending deletion"));
}

fn assert_gateway_pending_roots_on_pic(
    pic: &Pic,
    probe_id: Principal,
    gateway: Principal,
    expected_roots: &[&str],
) {
    let pending: Vec<String> =
        pic.query_call_as_or_panic(probe_id, gateway, BLOB_STORAGE_BLOBS_TO_DELETE, ());
    assert_eq!(
        pending,
        expected_roots
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
}

fn billing_status(
    pic: &Pic,
    probe_id: Principal,
    sync_gateway_principals: bool,
) -> BlobStorageStatusResponse {
    let status: Result<BlobStorageStatusResponse, Error> = pic.update_call_or_panic(
        probe_id,
        BLOB_STORAGE_STATUS,
        (BlobStorageStatusRequest {
            sync_gateway_principals,
        },),
    );
    status.expect("status endpoint should succeed")
}

fn assert_missing_billing_config_status(status: &BlobStorageStatusResponse, gateways: u64) {
    assert_eq!(
        status.payment_model,
        BlobStoragePaymentModelStatus::NotConfigured
    );
    assert_eq!(status.cashier_canister_id, None);
    assert_eq!(status.payment_account, None);
    assert_eq!(status.cashier_balance, None);
    assert_eq!(status.min_upload_balance, None);
    assert_eq!(status.target_upload_balance, None);
    assert_eq!(status.project_cycles_reserve, None);
    assert_eq!(status.gateway_principal_count, gateways);
    assert_eq!(status.last_gateway_principal_sync_at_ns, None);
    assert_eq!(
        status.gateway_principal_sync_action,
        BlobStorageGatewayPrincipalSyncAction::SkippedConfigMissing
    );
    assert_eq!(
        status.funding_status,
        BlobStorageFundingStatus::NotConfigured
    );
    assert!(!status.ready);
    assert_eq!(
        status.blockers,
        vec![BlobStorageReadinessBlocker::NotConfigured]
    );
    assert!(status.warnings.is_empty());
}

fn assert_billing_status_reports_missing_gateways(pic: &Pic, probe_id: Principal) {
    let status = billing_status(pic, probe_id, false);

    assert_eq!(status.gateway_principal_count, 0);
    assert_eq!(
        status.gateway_principal_sync_action,
        BlobStorageGatewayPrincipalSyncAction::NotRequested
    );
    assert_eq!(status.cashier_balance, Some(candid::Nat::from(123_u64)));
    assert_eq!(status.funding_status, BlobStorageFundingStatus::NotNeeded);
    assert!(!status.ready);
    assert_eq!(
        status.blockers,
        vec![BlobStorageReadinessBlocker::GatewayPrincipalsMissing]
    );
    assert_eq!(
        status.warnings,
        vec![BlobStorageBillingWarning::GatewayPrincipalSetEmpty]
    );
}

fn assert_billing_status_reports_funding_required(pic: &Pic, probe_id: Principal) {
    let status = billing_status(pic, probe_id, false);

    assert_eq!(status.gateway_principal_count, 1);
    assert_eq!(status.cashier_balance, Some(candid::Nat::from(9_u64)));
    assert_eq!(
        status.funding_status,
        BlobStorageFundingStatus::FundingRequired {
            requested_cycles: candid::Nat::from(91_u64),
        }
    );
    assert!(!status.ready);
    assert_eq!(
        status.blockers,
        vec![BlobStorageReadinessBlocker::InsufficientCashierBalance]
    );
    assert!(status.warnings.is_empty());
}

fn assert_billing_status_reports_reserve_violation(pic: &Pic, probe_id: Principal) {
    let status = billing_status(pic, probe_id, false);

    assert_eq!(status.cashier_balance, Some(candid::Nat::from(9_u64)));
    match status.funding_status {
        BlobStorageFundingStatus::ReserveWouldBeViolated {
            requested_cycles,
            transferable_cycles,
        } => {
            assert_eq!(requested_cycles, candid::Nat::from(91_u64));
            assert!(
                transferable_cycles < 91_u64,
                "reserve violation should report fewer transferable cycles than requested"
            );
        }
        other => panic!("expected reserve violation status, got {other:?}"),
    }
    assert!(!status.ready);
    assert_eq!(
        status.blockers,
        vec![
            BlobStorageReadinessBlocker::InsufficientCashierBalance,
            BlobStorageReadinessBlocker::ReserveWouldBeViolated,
        ]
    );
    assert!(status.warnings.is_empty());
}

fn status_project_cycles_available(status: &BlobStorageStatusResponse) -> u128 {
    nat_to_u128(&status.project_cycles_available)
}

fn assert_successful_funding_report(
    report: &BlobProjectCyclesTopUpReport,
    requested_cycles: u64,
    reserve_cycles: u64,
    cashier_total_after: u64,
) {
    assert_eq!(report.requested_cycles, candid::Nat::from(requested_cycles));
    assert_eq!(report.attached_cycles, candid::Nat::from(requested_cycles));
    assert_eq!(report.reserve_cycles, candid::Nat::from(reserve_cycles));
    assert_eq!(
        report.cashier_total_after,
        candid::Nat::from(cashier_total_after)
    );
    assert_eq!(report.skipped_reason, None);
    let project_cycles_before = nat_to_u128(&report.project_cycles_before);
    let project_cycles_after = nat_to_u128(&report.project_cycles_after);
    assert!(
        project_cycles_before > project_cycles_after,
        "project cycle balance must decrease during explicit funding"
    );
    assert!(
        project_cycles_before - project_cycles_after >= u128::from(requested_cycles),
        "project cycle balance decrease must cover the attached funding cycles"
    );
}

fn assert_reserve_skipped_funding_report(
    report: &BlobProjectCyclesTopUpReport,
    requested_cycles: u128,
    reserve_cycles: u128,
) {
    assert_eq!(report.requested_cycles, candid::Nat::from(requested_cycles));
    assert_eq!(report.attached_cycles, candid::Nat::from(0_u64));
    assert_eq!(report.reserve_cycles, candid::Nat::from(reserve_cycles));
    assert_eq!(report.cashier_total_after, candid::Nat::from(0_u64));
    assert_eq!(report.project_cycles_before, report.project_cycles_after);
    assert_eq!(
        report.skipped_reason.as_deref(),
        Some("reserve would be violated")
    );
}

fn nat_to_u128(value: &candid::Nat) -> u128 {
    u128::try_from(value.0.clone()).expect("Candid nat should fit u128")
}

fn assert_billing_status_matches_config(
    status: &BlobStorageStatusResponse,
    cashier_id: Principal,
    probe_id: Principal,
    project_cycles_reserve: u64,
    gateway_principal_count: u64,
) -> u64 {
    assert_eq!(
        status.payment_model,
        BlobStoragePaymentModelStatus::ProjectAsPaymentAccount
    );
    assert_eq!(status.cashier_canister_id, Some(cashier_id));
    assert_eq!(status.payment_account, Some(probe_id));
    assert_eq!(status.cashier_balance, Some(candid::Nat::from(123_u64)));
    assert_eq!(status.min_upload_balance, Some(candid::Nat::from(10_u64)));
    assert_eq!(
        status.target_upload_balance,
        Some(candid::Nat::from(100_u64))
    );
    assert_eq!(
        status.project_cycles_reserve,
        Some(candid::Nat::from(project_cycles_reserve))
    );
    assert_eq!(status.gateway_principal_count, gateway_principal_count);
    assert_eq!(
        status.gateway_principal_sync_action,
        BlobStorageGatewayPrincipalSyncAction::NotRequested
    );
    assert_eq!(status.funding_status, BlobStorageFundingStatus::NotNeeded);
    assert!(status.ready);
    assert!(status.blockers.is_empty());
    assert!(status.warnings.is_empty());
    status
        .last_gateway_principal_sync_at_ns
        .expect("successful gateway sync timestamp should be recorded")
}

fn assert_initial_gateway_sync_succeeds(pic: &Pic, probe_id: Principal) {
    let synced: Result<(), Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS, ());
    synced.expect("gateway sync should succeed");

    let counts: Result<BlobStorageLocalCounters, Error> =
        pic.query_call_or_panic(probe_id, "blob_storage_probe_counts", ());
    assert_eq!(
        counts.expect("probe counts query should succeed"),
        BlobStorageLocalCounters::new(0, 0, 1)
    );
}

fn assert_mock_failure_controls_require_controller(pic: &Pic, cashier_id: Principal) {
    let non_controller = principal(0x91);

    let balance_denied: Result<(), Error> = pic.update_call_as_or_panic(
        cashier_id,
        non_controller,
        "blob_storage_cashier_mock_set_next_balance_error",
        (Some(
            BlobStorageCashierAccountBalanceGetError::InternalError(
                "denied balance failure".to_string(),
            ),
        ),),
    );
    assert_eq!(
        balance_denied
            .expect_err("non-controller must not configure mock balance failures")
            .code,
        ErrorCode::Unauthorized
    );

    let top_up_denied: Result<(), Error> = pic.update_call_as_or_panic(
        cashier_id,
        non_controller,
        "blob_storage_cashier_mock_set_next_top_up_error",
        (Some(
            BlobStorageCashierAccountTopUpError::TopUpWithoutCycles,
        ),),
    );
    assert_eq!(
        top_up_denied
            .expect_err("non-controller must not configure mock top-up failures")
            .code,
        ErrorCode::Unauthorized
    );

    let balance_total_denied: Result<(), Error> = pic.update_call_as_or_panic(
        cashier_id,
        non_controller,
        "blob_storage_cashier_mock_set_next_balance_total",
        (Some(candid::Int::from(-1)),),
    );
    assert_eq!(
        balance_total_denied
            .expect_err("non-controller must not configure mock malformed balance responses")
            .code,
        ErrorCode::Unauthorized
    );

    let top_up_total_denied: Result<(), Error> = pic.update_call_as_or_panic(
        cashier_id,
        non_controller,
        "blob_storage_cashier_mock_set_next_top_up_total",
        (Some(candid::Int::from(-1)),),
    );
    assert_eq!(
        top_up_total_denied
            .expect_err("non-controller must not configure mock malformed top-up responses")
            .code,
        ErrorCode::Unauthorized
    );

    let gateway_list_trap_denied: Result<(), Error> = pic.update_call_as_or_panic(
        cashier_id,
        non_controller,
        "blob_storage_cashier_mock_set_gateway_list_trap",
        (Some("denied gateway-list trap".to_string()),),
    );
    assert_eq!(
        gateway_list_trap_denied
            .expect_err("non-controller must not configure mock gateway-list traps")
            .code,
        ErrorCode::Unauthorized
    );
}

fn assert_gateway_sync_rejects_invalid_cashier_list_without_mutation(
    pic: &Pic,
    cashier_id: Principal,
    probe_id: Principal,
) {
    let sync_at_before = billing_status(pic, probe_id, false)
        .last_gateway_principal_sync_at_ns
        .expect("successful gateway sync timestamp should exist before failed sync");

    let seeded_gateways: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_gateways",
        (Vec::<Principal>::new(),),
    );
    seeded_gateways.expect("mock Cashier empty gateway seed should succeed");

    let synced: Result<(), Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS, ());
    assert_eq!(
        synced
            .expect_err("empty Cashier gateway list should fail sync")
            .code,
        ErrorCode::InternalRpcMalformed
    );

    assert_failed_gateway_sync_preserves_state(pic, probe_id, sync_at_before, "empty gateway sync");

    let seeded_gateways: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_gateways",
        (vec![Principal::anonymous()],),
    );
    seeded_gateways.expect("mock Cashier invalid gateway seed should succeed");

    let synced: Result<(), Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS, ());
    assert_eq!(
        synced
            .expect_err("invalid Cashier gateway list should fail sync")
            .code,
        ErrorCode::InternalRpcMalformed
    );

    assert_failed_gateway_sync_preserves_state(
        pic,
        probe_id,
        sync_at_before,
        "invalid gateway sync",
    );

    let configured: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_gateway_list_trap",
        (Some("mock gateway list trap".to_string()),),
    );
    configured.expect("mock Cashier gateway-list trap should be configured");

    let synced: Result<(), Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS, ());
    assert_eq!(
        synced
            .expect_err("trapped Cashier gateway list should fail sync")
            .code,
        ErrorCode::Internal
    );

    assert_failed_gateway_sync_preserves_state(
        pic,
        probe_id,
        sync_at_before,
        "trapped gateway sync",
    );

    let cleared: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_gateway_list_trap",
        (Option::<String>::None,),
    );
    cleared.expect("mock Cashier gateway-list trap should be cleared");

    let replacement_gateway = principal(0x5a);
    let seeded_gateways: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_gateways",
        (vec![replacement_gateway],),
    );
    seeded_gateways.expect("mock Cashier replacement gateway seed should succeed");

    pic.advance_time(Duration::from_nanos(1));
    pic.tick();

    let synced: Result<(), Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS, ());
    synced.expect("gateway sync should recover after cleared trap");

    let status = billing_status(pic, probe_id, false);
    let sync_at_after = status
        .last_gateway_principal_sync_at_ns
        .expect("recovered gateway sync timestamp should be recorded");
    assert!(
        sync_at_after > sync_at_before,
        "successful sync after a cleared trap must record a fresh timestamp"
    );
    assert_eq!(status.gateway_principal_count, 1);
}

fn assert_failed_gateway_sync_preserves_state(
    pic: &Pic,
    probe_id: Principal,
    sync_at_before: u64,
    context: &str,
) {
    let counts: Result<BlobStorageLocalCounters, Error> =
        pic.query_call_or_panic(probe_id, "blob_storage_probe_counts", ());
    assert_eq!(
        counts.expect("probe counts query should succeed"),
        BlobStorageLocalCounters::new(0, 0, 1),
        "{context} must leave the previous gateway set intact"
    );

    let status = billing_status(pic, probe_id, false);
    assert_eq!(
        status.last_gateway_principal_sync_at_ns,
        Some(sync_at_before),
        "{context} must not record a successful gateway sync timestamp"
    );
}

fn assert_billing_status_reports_cashier_balance_malformed(
    pic: &Pic,
    cashier_id: Principal,
    probe_id: Principal,
) {
    let configured: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_next_balance_total",
        (Some(candid::Int::from(-1)),),
    );
    configured.expect("mock Cashier malformed balance should be configured");

    let status: Result<BlobStorageStatusResponse, Error> = pic.update_call_or_panic(
        probe_id,
        BLOB_STORAGE_STATUS,
        (BlobStorageStatusRequest {
            sync_gateway_principals: true,
        },),
    );
    let status = status.expect("status endpoint should succeed");

    assert_eq!(status.cashier_balance, None);
    assert_eq!(
        status.funding_status,
        BlobStorageFundingStatus::BalanceMalformed
    );
    assert!(!status.ready);
    assert!(
        status
            .blockers
            .contains(&BlobStorageReadinessBlocker::CashierBalanceMalformed)
    );
    assert!(
        status
            .warnings
            .contains(&BlobStorageBillingWarning::CashierBalanceMalformed)
    );
}

fn assert_billing_status_reports_cashier_balance_unavailable(
    pic: &Pic,
    cashier_id: Principal,
    probe_id: Principal,
) {
    let configured: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_next_balance_error",
        (Some(
            BlobStorageCashierAccountBalanceGetError::InternalError(
                "mock balance failure".to_string(),
            ),
        ),),
    );
    configured.expect("mock Cashier balance failure should be configured");

    let status: Result<BlobStorageStatusResponse, Error> = pic.update_call_or_panic(
        probe_id,
        BLOB_STORAGE_STATUS,
        (BlobStorageStatusRequest {
            sync_gateway_principals: true,
        },),
    );
    let status = status.expect("status endpoint should succeed");

    assert_eq!(status.cashier_balance, None);
    assert_eq!(
        status.funding_status,
        BlobStorageFundingStatus::BalanceUnavailable
    );
    assert!(!status.ready);
    assert!(
        status
            .blockers
            .contains(&BlobStorageReadinessBlocker::CashierBalanceUnavailable)
    );
    assert!(
        status
            .warnings
            .contains(&BlobStorageBillingWarning::CashierBalanceUnavailable)
    );
}

fn assert_cashier_top_up_error_maps_to_public_code(
    pic: &Pic,
    cashier_id: Principal,
    probe_id: Principal,
    error: BlobStorageCashierAccountTopUpError,
    expected_code: ErrorCode,
) {
    let configured: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_next_top_up_error",
        (Some(error),),
    );
    configured.expect("mock Cashier top-up failure should be configured");

    let top_up: Result<BlobProjectCyclesTopUpReport, Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, (11_u128,));

    assert_eq!(
        top_up
            .expect_err("mock Cashier top-up failure should propagate")
            .code,
        expected_code
    );

    let last_top_up: Result<MockCashierLastTopUp, Error> =
        pic.query_call_or_panic(cashier_id, "blob_storage_cashier_mock_last_top_up", ());
    assert_eq!(
        last_top_up.expect("last top-up query should succeed"),
        None,
        "forced Cashier errors must not be recorded as successful top-ups"
    );
}

fn assert_cashier_top_up_malformed_balance_maps_to_rpc_malformed(
    pic: &Pic,
    cashier_id: Principal,
    probe_id: Principal,
) {
    let configured: Result<(), Error> = pic.update_call_or_panic(
        cashier_id,
        "blob_storage_cashier_mock_set_next_top_up_total",
        (Some(candid::Int::from(-1)),),
    );
    configured.expect("mock Cashier malformed top-up balance should be configured");

    let top_up: Result<BlobProjectCyclesTopUpReport, Error> =
        pic.update_call_or_panic(probe_id, BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, (11_u128,));
    assert_eq!(
        top_up
            .expect_err("malformed Cashier top-up balance should propagate")
            .code,
        ErrorCode::InternalRpcMalformed
    );

    let last_top_up: Result<MockCashierLastTopUp, Error> =
        pic.query_call_or_panic(cashier_id, "blob_storage_cashier_mock_last_top_up", ());
    assert_eq!(
        last_top_up.expect("last top-up query should succeed"),
        None,
        "malformed Cashier top-up responses must not be recorded as successful top-ups"
    );
}

fn assert_reserve_violation_does_not_partially_top_up(
    pic: &Pic,
    cashier_id: Principal,
    probe_id: Principal,
) {
    let status: Result<BlobStorageStatusResponse, Error> = pic.update_call_or_panic(
        probe_id,
        BLOB_STORAGE_STATUS,
        (BlobStorageStatusRequest {
            sync_gateway_principals: false,
        },),
    );
    let project_cycles_available = u128::try_from(
        status
            .expect("status should report available project cycles")
            .project_cycles_available
            .0,
    )
    .expect("available project cycles should fit u128");
    let transferable_cycles = 1_000_u128;
    assert!(
        project_cycles_available > transferable_cycles,
        "probe should have enough cycles to exercise partial-reserve refusal"
    );

    configure_billing_with_reserve(
        pic,
        cashier_id,
        probe_id,
        project_cycles_available - transferable_cycles,
    );

    let top_up: Result<BlobProjectCyclesTopUpReport, Error> = pic.update_call_or_panic(
        probe_id,
        BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
        (transferable_cycles + 1,),
    );
    let report = top_up.expect("reserve-blocked funding should return a skipped report");
    assert_reserve_skipped_funding_report(
        &report,
        transferable_cycles + 1,
        project_cycles_available - transferable_cycles,
    );

    let last_top_up: Result<MockCashierLastTopUp, Error> =
        pic.query_call_or_panic(cashier_id, "blob_storage_cashier_mock_last_top_up", ());
    assert_eq!(
        last_top_up.expect("last top-up query should succeed"),
        None,
        "reserve-blocked funding must not create a partial Cashier top-up"
    );

    configure_billing(pic, cashier_id, probe_id);
}

fn install_billing_canisters(pic: &Pic) -> (Principal, Principal) {
    let cashier_id = install_standalone_canister_on_pic(
        pic,
        CASHIER_MOCK_CRATE,
        PROBE_ROLE,
        CanicWasmBuildProfile::Fast,
        "blob-storage-cashier-mock",
    );
    let probe_id = install_probe_canister(pic);
    (cashier_id, probe_id)
}

fn install_probe_canister(pic: &Pic) -> Principal {
    install_standalone_canister_on_pic(
        pic,
        PROBE_CRATE,
        PROBE_ROLE,
        CanicWasmBuildProfile::Fast,
        "blob-storage-probe",
    )
}

fn configure_billing(pic: &Pic, cashier_id: Principal, probe_id: Principal) {
    configure_billing_with_reserve(pic, cashier_id, probe_id, 1);
}

fn configure_billing_with_reserve(
    pic: &Pic,
    cashier_id: Principal,
    probe_id: Principal,
    project_cycles_reserve: u128,
) {
    let configured: Result<(), Error> = pic.update_call_or_panic(
        probe_id,
        "blob_storage_probe_configure_billing",
        (BlobStorageBillingConfig {
            cashier_canister_id: cashier_id,
            project_cycles_reserve: candid::Nat::from(project_cycles_reserve),
            min_upload_balance: candid::Nat::from(10_u64),
            target_upload_balance: candid::Nat::from(100_u64),
            gateway_principal_limit: 8,
        },),
    );
    configured.expect("probe billing config should be accepted");
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
    upgrade_probe_canister_on_pic(fixture.pic(), fixture.canister_id());
}

// Upgrade the probe with the same compiled wasm artifact used for install.
fn upgrade_probe_canister_on_pic(pic: &Pic, canister_id: Principal) {
    pic.wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    let wasm = probe_wasm();

    pic.retry_install_code_ok(INSTALL_CODE_RETRY_LIMIT, INSTALL_CODE_COOLDOWN, || {
        pic.upgrade_canister(canister_id, wasm.clone(), upgrade_args(), None)
            .map_err(|err| err.to_string())
    })
    .expect("probe upgrade should succeed");
    pic.wait_for_ready(canister_id, READY_TICK_LIMIT, "blob storage post_upgrade");
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
