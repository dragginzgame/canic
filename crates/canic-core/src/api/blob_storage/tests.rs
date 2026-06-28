use super::BlobStorageApi;
#[cfg(feature = "blob-storage-billing")]
use super::billing::BlobStorageFundingAttachment;
use crate::{
    cdk::types::Principal,
    dto::{
        blob_storage::{BlobStorageLocalCounters, CreateCertificateResult},
        error::ErrorCode,
    },
};

#[cfg(feature = "blob-storage-billing")]
use crate::{
    cdk::candid::Nat,
    dto::blob_storage::{
        BlobStorageBillingConfig, BlobStorageCashierAccountTopUpError, BlobStorageFundingStatus,
        BlobStorageGatewayPrincipalSyncAction, BlobStorageReadinessBlocker,
        BlobStorageStatusRequest,
    },
    ops::{
        blob_storage::funding::BlobStorageFundingInProgress,
        cashier::conversion::CashierDecodeError,
    },
};

#[cfg(feature = "blob-storage-billing")]
fn billing_test_principal(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

#[cfg(feature = "blob-storage-billing")]
fn billing_test_config(
    cashier_canister_id: Principal,
    project_cycles_reserve: u128,
    min_upload_balance: u128,
    target_upload_balance: u128,
    gateway_principal_limit: u64,
) -> BlobStorageBillingConfig {
    BlobStorageBillingConfig {
        cashier_canister_id,
        project_cycles_reserve: Nat::from(project_cycles_reserve),
        min_upload_balance: Nat::from(min_upload_balance),
        target_upload_balance: Nat::from(target_upload_balance),
        gateway_principal_limit,
    }
}

#[cfg(feature = "blob-storage-billing")]
fn oversized_billing_nat() -> Nat {
    Nat::parse("340282366920938463463374607431768211456".as_bytes())
        .expect("valid oversized Candid nat")
}

#[test]
fn canonical_root_hash_text_normalizes_toko_hashes() {
    let hash = BlobStorageApi::canonical_root_hash_text(
        "sha256:ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD",
    )
    .expect("hash parses");

    assert_eq!(
        hash,
        "sha256:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
    );
}

#[test]
fn canonical_root_hash_bytes_matches_gateway_query_shape() {
    let hash =
        BlobStorageApi::canonical_root_hash_bytes(&[0xabu8; 32]).expect("hash bytes convert");

    assert_eq!(
        hash,
        "sha256:abababababababababababababababababababababababababababababababab"
    );
}

#[test]
fn malformed_root_hash_maps_to_public_invalid_input() {
    let err = BlobStorageApi::canonical_root_hash_text("sha256:zz")
        .expect_err("short malformed hash should fail");

    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn create_certificate_echoes_request_hash_and_registers_canonical_root() {
    crate::storage::stable::blob_storage::BlobStorageStore::clear();
    let request_hash =
        "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string();
    let canonical_hash = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    let result = BlobStorageApi::create_certificate(request_hash.clone())
        .expect("create certificate succeeds");

    assert_eq!(
        result,
        CreateCertificateResult {
            method: "upload".to_string(),
            blob_hash: request_hash
        }
    );
    assert!(BlobStorageApi::is_live(canonical_hash).expect("canonical live check"));
    assert_eq!(
        BlobStorageApi::blobs_are_live(vec![vec![0xaau8; 32]]),
        vec![true]
    );
}

#[test]
fn repeated_create_certificate_is_canonical_idempotent() {
    crate::storage::stable::blob_storage::BlobStorageStore::clear();
    let upper =
        "sha256:BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string();
    let lower =
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string();

    let first = BlobStorageApi::create_certificate(upper.clone()).expect("first create");
    let second = BlobStorageApi::create_certificate(lower.clone()).expect("second create");

    assert_eq!(first.blob_hash, upper);
    assert_eq!(second.blob_hash, lower);
    assert_eq!(BlobStorageApi::stored_blob_count(), 1);
    assert_eq!(BlobStorageApi::pending_deletion_count(), 0);
    assert!(BlobStorageApi::is_live(&lower).expect("canonical live check"));
}

#[test]
fn malformed_api_inputs_do_not_mutate_blob_state() {
    crate::storage::stable::blob_storage::BlobStorageStore::clear();
    let malformed = "sha256:zz";

    assert_eq!(
        BlobStorageApi::create_certificate(malformed.to_string())
            .expect_err("malformed create fails")
            .code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        BlobStorageApi::register_live(malformed, 10)
            .expect_err("malformed register fails")
            .code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        BlobStorageApi::is_live(malformed)
            .expect_err("malformed live check fails")
            .code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        BlobStorageApi::mark_pending_delete(malformed, 20)
            .expect_err("malformed pending delete fails")
            .code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        BlobStorageApi::confirm_deleted_by_gateway_hash_bytes(&[0u8; 31])
            .expect_err("malformed gateway confirm fails")
            .code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        BlobStorageApi::local_counters(),
        BlobStorageLocalCounters::new(0, 0, 0)
    );
    assert!(BlobStorageApi::pending_deletion_hashes().is_empty());
}

#[test]
fn live_blob_lifecycle_maps_to_public_api() {
    crate::storage::stable::blob_storage::BlobStorageStore::clear();
    let hash = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    assert!(!BlobStorageApi::is_live(hash).expect("live check"));
    assert_eq!(BlobStorageApi::stored_blob_count(), 0);
    assert_eq!(BlobStorageApi::pending_deletion_count(), 0);
    assert_eq!(
        BlobStorageApi::require_live(hash)
            .expect_err("missing blob is not live")
            .code,
        ErrorCode::NotFound
    );
    assert!(BlobStorageApi::register_live(hash, 10).expect("register"));
    assert!(!BlobStorageApi::register_live(hash, 20).expect("register again"));
    assert!(BlobStorageApi::is_live(hash).expect("live check"));
    assert_eq!(BlobStorageApi::stored_blob_count(), 1);
    assert_eq!(BlobStorageApi::pending_deletion_count(), 0);
    BlobStorageApi::require_live(hash).expect("require live");

    assert!(BlobStorageApi::mark_pending_delete(hash, 30).expect("mark pending"));
    assert!(!BlobStorageApi::mark_pending_delete(hash, 40).expect("mark again"));
    assert_eq!(BlobStorageApi::stored_blob_count(), 1);
    assert_eq!(BlobStorageApi::pending_deletion_count(), 1);
    assert_eq!(
        BlobStorageApi::local_counters(),
        BlobStorageLocalCounters::new(1, 1, 0)
    );
    assert_eq!(
        BlobStorageApi::require_live(hash)
            .expect_err("pending is not live")
            .code,
        ErrorCode::Conflict
    );
}

#[test]
fn gateway_byte_confirmation_removes_live_blob() {
    crate::storage::stable::blob_storage::BlobStorageStore::clear();
    let hash = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    let bytes = [0xddu8; 32];

    BlobStorageApi::register_live(hash, 10).expect("register");
    BlobStorageApi::mark_pending_delete(hash, 20).expect("mark pending");
    assert_eq!(
        BlobStorageApi::pending_deletion_hashes(),
        vec![hash.to_string()]
    );

    BlobStorageApi::confirm_deleted_by_gateway_hash_bytes(&bytes).expect("confirm");

    assert!(!BlobStorageApi::is_live(hash).expect("live check"));
    assert!(BlobStorageApi::pending_deletion_hashes().is_empty());
}

#[test]
fn gateway_principal_api_is_idempotent() {
    let principal = Principal::from_slice(&[99; 29]);

    crate::storage::stable::blob_storage::BlobStorageStore::clear();
    assert!(!BlobStorageApi::is_gateway_principal(principal));
    assert_eq!(BlobStorageApi::gateway_principal_count(), 0);

    BlobStorageApi::upsert_gateway_principal(principal, 10);
    assert!(BlobStorageApi::is_gateway_principal(principal));
    assert_eq!(BlobStorageApi::gateway_principal_count(), 1);
    assert_eq!(
        BlobStorageApi::local_counters(),
        BlobStorageLocalCounters::new(0, 0, 1)
    );
    assert!(BlobStorageApi::remove_gateway_principal(principal));
    assert!(!BlobStorageApi::remove_gateway_principal(principal));
    assert_eq!(BlobStorageApi::gateway_principal_count(), 0);
}

#[test]
fn gateway_endpoint_helpers_match_toko_malformed_input_behavior() {
    crate::storage::stable::blob_storage::BlobStorageStore::clear();
    let hash = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    let bytes = [0xeeu8; 32];
    let gateway = Principal::from_slice(&[11; 29]);

    assert_eq!(
        BlobStorageApi::blobs_are_live(vec![bytes.to_vec(), vec![1, 2, 3]]),
        vec![false, false]
    );

    BlobStorageApi::create_certificate(hash.to_string()).expect("create certificate");
    assert_eq!(
        BlobStorageApi::blobs_are_live(vec![bytes.to_vec(), vec![1, 2, 3]]),
        vec![true, false]
    );

    BlobStorageApi::mark_pending_delete(hash, 10).expect("mark pending");
    assert!(BlobStorageApi::pending_deletion_hashes_for_gateway(gateway).is_empty());
    BlobStorageApi::confirm_deleted_by_gateway_hash_bytes_batch(gateway, vec![bytes.to_vec()]);
    assert_eq!(
        BlobStorageApi::pending_deletion_hashes(),
        vec![hash.to_string()]
    );

    BlobStorageApi::upsert_gateway_principal(gateway, 20);
    assert_eq!(
        BlobStorageApi::pending_deletion_hashes_for_gateway(gateway),
        vec![hash.to_string()]
    );

    BlobStorageApi::confirm_deleted_by_gateway_hash_bytes_batch(
        gateway,
        vec![vec![1, 2, 3], bytes.to_vec()],
    );

    assert!(BlobStorageApi::pending_deletion_hashes().is_empty());
    assert!(!BlobStorageApi::is_live(hash).expect("live check"));
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn cashier_decode_errors_map_to_rpc_malformed_code() {
    let empty_gateway =
        BlobStorageApi::map_cashier_decode_error(CashierDecodeError::EmptyGatewayPrincipalList);
    assert_eq!(empty_gateway.code, ErrorCode::InternalRpcMalformed);

    let invalid_balance =
        BlobStorageApi::map_cashier_decode_error(CashierDecodeError::InvalidCycleBalance {
            field: "total",
        });
    assert_eq!(invalid_balance.code, ErrorCode::InternalRpcMalformed);

    let invalid_gateway =
        BlobStorageApi::map_cashier_decode_error(CashierDecodeError::InvalidGatewayPrincipal {
            principal: Principal::anonymous(),
        });
    assert_eq!(invalid_gateway.code, ErrorCode::InternalRpcMalformed);

    let too_many_gateways =
        BlobStorageApi::map_cashier_decode_error(CashierDecodeError::TooManyGatewayPrincipals {
            actual: 2,
            max: 1,
        });
    assert_eq!(too_many_gateways.code, ErrorCode::InternalRpcMalformed);
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn cashier_top_up_errors_map_to_stable_public_codes() {
    let unauthorized = BlobStorageApi::map_cashier_top_up_error(
        BlobStorageCashierAccountTopUpError::NotAuthorized(Principal::from_slice(&[1; 29])),
    );
    assert_eq!(unauthorized.code, ErrorCode::Forbidden);

    let overflow = BlobStorageApi::map_cashier_top_up_error(
        BlobStorageCashierAccountTopUpError::AccountBalanceOverflow,
    );
    assert_eq!(overflow.code, ErrorCode::ResourceExhausted);

    let internal = BlobStorageApi::map_cashier_top_up_error(
        BlobStorageCashierAccountTopUpError::InternalError("down".to_string()),
    );
    assert_eq!(internal.code, ErrorCode::Internal);

    let without_cycles = BlobStorageApi::map_cashier_top_up_error(
        BlobStorageCashierAccountTopUpError::TopUpWithoutCycles,
    );
    assert_eq!(without_cycles.code, ErrorCode::InvalidInput);
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn funding_in_progress_maps_to_conflict_code() {
    let err = BlobStorageApi::map_funding_in_progress(BlobStorageFundingInProgress);

    assert_eq!(err.code, ErrorCode::Conflict);
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn validate_requested_funding_cycles_rejects_zero() {
    let err = BlobStorageApi::validate_requested_funding_cycles(0)
        .expect_err("zero requested cycles should be invalid");

    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn configure_billing_rejects_invalid_config_without_replacing_current_config() {
    crate::storage::stable::blob_storage::BlobStorageStore::clear_billing();
    let valid = billing_test_config(billing_test_principal(1), 1, 10, 100, 8);
    BlobStorageApi::configure_billing(valid.clone()).expect("valid billing config is stored");

    let invalid_configs = [
        billing_test_config(Principal::anonymous(), 1, 10, 100, 8),
        billing_test_config(Principal::management_canister(), 1, 10, 100, 8),
        billing_test_config(billing_test_principal(2), 0, 10, 100, 8),
        billing_test_config(billing_test_principal(3), 1, 0, 100, 8),
        billing_test_config(billing_test_principal(4), 1, 10, 0, 8),
        billing_test_config(billing_test_principal(5), 1, 100, 10, 8),
        billing_test_config(billing_test_principal(6), 1, 10, 100, 0),
    ];

    for config in invalid_configs {
        let err = BlobStorageApi::configure_billing(config)
            .expect_err("invalid billing config should be rejected");
        assert_eq!(err.code, ErrorCode::InvalidInput);
        assert_eq!(BlobStorageApi::billing_config(), Some(valid.clone()));
    }
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn configure_billing_rejects_oversized_nat_fields_without_replacing_current_config() {
    crate::storage::stable::blob_storage::BlobStorageStore::clear_billing();
    let valid = billing_test_config(billing_test_principal(1), 1, 10, 100, 8);
    BlobStorageApi::configure_billing(valid.clone()).expect("valid billing config is stored");

    let mut oversized_reserve = valid.clone();
    oversized_reserve.project_cycles_reserve = oversized_billing_nat();

    let mut oversized_min_upload = valid.clone();
    oversized_min_upload.min_upload_balance = oversized_billing_nat();

    let mut oversized_target_upload = valid.clone();
    oversized_target_upload.target_upload_balance = oversized_billing_nat();

    for config in [
        oversized_reserve,
        oversized_min_upload,
        oversized_target_upload,
    ] {
        let err = BlobStorageApi::configure_billing(config)
            .expect_err("oversized billing nat should be rejected");
        assert_eq!(err.code, ErrorCode::InvalidInput);
        assert_eq!(BlobStorageApi::billing_config(), Some(valid.clone()));
    }
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn funding_attachment_attaches_requested_cycles_when_reserve_allows() {
    assert_eq!(
        BlobStorageApi::funding_attachment(500, 1_000, 500),
        BlobStorageFundingAttachment {
            project_cycles_available: 1_000,
            attached_cycles: 500,
            skipped_reason: None,
        }
    );
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn funding_attachment_rejects_partial_top_up_when_reserve_would_be_violated() {
    assert_eq!(
        BlobStorageApi::funding_attachment(500, 1_000, 700),
        BlobStorageFundingAttachment {
            project_cycles_available: 1_000,
            attached_cycles: 0,
            skipped_reason: Some("reserve would be violated"),
        }
    );
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn status_sync_action_is_read_only_when_requested() {
    assert_eq!(
        BlobStorageApi::status_sync_action(
            &BlobStorageStatusRequest {
                sync_gateway_principals: false,
            },
            true,
        ),
        BlobStorageGatewayPrincipalSyncAction::NotRequested
    );
    assert_eq!(
        BlobStorageApi::status_sync_action(
            &BlobStorageStatusRequest {
                sync_gateway_principals: true,
            },
            false,
        ),
        BlobStorageGatewayPrincipalSyncAction::SkippedConfigMissing
    );
    assert_eq!(
        BlobStorageApi::status_sync_action(
            &BlobStorageStatusRequest {
                sync_gateway_principals: true,
            },
            true,
        ),
        BlobStorageGatewayPrincipalSyncAction::SkippedReadOnlyStatus
    );
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn status_funding_status_reports_not_needed_at_min_balance() {
    let mut blockers = Vec::new();

    let status = BlobStorageApi::status_funding_status(10, 10, 100, 1, 1_000, &mut blockers);

    assert_eq!(status, BlobStorageFundingStatus::NotNeeded);
    assert!(blockers.is_empty());
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn status_funding_status_reports_required_top_up() {
    let mut blockers = Vec::new();

    let status = BlobStorageApi::status_funding_status(9, 10, 100, 1, 1_000, &mut blockers);

    assert_eq!(
        status,
        BlobStorageFundingStatus::FundingRequired {
            requested_cycles: Nat::from(91_u64),
        }
    );
    assert_eq!(
        blockers,
        vec![BlobStorageReadinessBlocker::InsufficientCashierBalance]
    );
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn status_funding_status_reports_reserve_violation() {
    let mut blockers = Vec::new();

    let status = BlobStorageApi::status_funding_status(9, 10, 100, 950, 1_000, &mut blockers);

    assert_eq!(
        status,
        BlobStorageFundingStatus::ReserveWouldBeViolated {
            requested_cycles: Nat::from(91_u64),
            transferable_cycles: Nat::from(50_u64),
        }
    );
    assert_eq!(
        blockers,
        vec![
            BlobStorageReadinessBlocker::InsufficientCashierBalance,
            BlobStorageReadinessBlocker::ReserveWouldBeViolated,
        ]
    );
}
