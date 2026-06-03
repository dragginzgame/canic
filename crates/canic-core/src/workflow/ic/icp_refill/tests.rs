use super::*;

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn sample_record(status: IcpRefillStatus) -> IcpRefillRecord {
    IcpRefillRecord {
        id: 7,
        operation_id: [9; 32],
        source_canister: p(1),
        source_subaccount: Some([2; 32]),
        target_canister: p(3),
        ledger_canister_id: p(4),
        cmc_canister_id: p(5),
        cmc_to_account_owner: p(5),
        cmc_to_account_subaccount: Some([6; 32]),
        amount_e8s: 100_000_000,
        fee_e8s: 10_000,
        memo: IcpRefillOps::topup_memo(),
        created_at_time_ns: 1_000,
        ledger_block_index: None,
        notify_attempts: 0,
        cycles_sent: None,
        status,
        error_code: None,
        error_message: None,
        refund_block_index: None,
        transaction_too_old_min_block_index: None,
        created_at_ns: 1_000,
        updated_at_ns: 1_000,
    }
}

fn request_for(record: &IcpRefillRecord) -> IcpRefillRequest {
    IcpRefillRequest {
        operation_id: record.operation_id,
        source_canister: record.source_canister,
        source_subaccount: record.source_subaccount,
        target_canister: record.target_canister,
        amount_e8s: record.amount_e8s,
        dry_run: false,
        mode: IcpRefillMode::Canister,
    }
}

#[test]
fn in_flight_statuses_are_narrow() {
    assert!(is_in_flight(IcpRefillStatus::Requested));
    assert!(is_in_flight(IcpRefillStatus::Transferred));
    assert!(is_in_flight(IcpRefillStatus::NotifyProcessing));
    assert!(!is_in_flight(IcpRefillStatus::Completed));
    assert!(!is_in_flight(IcpRefillStatus::Failed));
    assert!(!is_in_flight(IcpRefillStatus::Refunded));
}

#[test]
fn fabricate_dry_run_message_is_loud() {
    assert_eq!(
        dry_run_message(IcpRefillMode::Fabricate),
        Some("mode=fabricate (does not call canister refill endpoint)".to_string())
    );
}

#[test]
fn canister_dry_run_message_is_empty() {
    assert_eq!(dry_run_message(IcpRefillMode::Canister), None);
}

#[test]
fn estimate_cycles_uses_icp_xdr_permyriad_units() {
    assert_eq!(
        estimate_cycles(100_000_000, 40_000).to_u128(),
        4_000_000_000_000
    );
    assert_eq!(
        estimate_cycles(50_000_000, 40_000).to_u128(),
        2_000_000_000_000
    );
}

#[test]
fn ledger_decimals_validation_requires_icp_decimals() {
    validate_ledger_decimals(8).expect("ICP decimals");
    let err = validate_ledger_decimals(6).expect_err("non-ICP decimals must fail");
    assert!(err.to_string().contains("decimals=8"));
}

#[test]
fn transfer_window_stale_uses_strict_tx_window() {
    let record = sample_record(IcpRefillStatus::Requested);

    assert!(!transfer_window_stale(
        &record,
        record.created_at_time_ns + TX_WINDOW_NANOS
    ));
    assert!(transfer_window_stale(
        &record,
        record.created_at_time_ns + TX_WINDOW_NANOS + 1
    ));
}

#[test]
fn transfer_window_stale_requires_requested_without_block_index() {
    let mut record = sample_record(IcpRefillStatus::Requested);
    record.ledger_block_index = Some(10);
    assert!(!transfer_window_stale(
        &record,
        record.created_at_time_ns + TX_WINDOW_NANOS + 1
    ));

    let record = sample_record(IcpRefillStatus::Failed);
    assert!(!transfer_window_stale(
        &record,
        record.created_at_time_ns + TX_WINDOW_NANOS + 1
    ));
}

#[test]
fn notify_retry_only_allows_notify_failed_with_block_index() {
    let mut record = sample_record(IcpRefillStatus::Failed);
    record.error_code = Some(IcpRefillErrorCode::NotifyFailed);
    record.ledger_block_index = Some(10);
    assert!(can_retry_notify(&record));
    assert!(should_notify(&record));

    record.ledger_block_index = None;
    assert!(!can_retry_notify(&record));
    assert!(!should_notify(&record));

    let mut transferred = sample_record(IcpRefillStatus::Transferred);
    transferred.ledger_block_index = Some(11);
    assert!(should_notify(&transferred));
}

#[test]
fn hub_self_refill_resumes_in_flight_and_retryable_records() {
    assert!(is_resumable(&sample_record(IcpRefillStatus::Requested)));
    assert!(is_resumable(&sample_record(IcpRefillStatus::Transferred)));
    assert!(is_resumable(&sample_record(
        IcpRefillStatus::NotifyProcessing
    )));

    let mut notify_failed = sample_record(IcpRefillStatus::Failed);
    notify_failed.error_code = Some(IcpRefillErrorCode::NotifyFailed);
    notify_failed.ledger_block_index = Some(11);
    assert!(is_resumable(&notify_failed));

    let mut bad_fee = sample_record(IcpRefillStatus::Failed);
    bad_fee.error_code = Some(IcpRefillErrorCode::BadFee);
    assert!(is_resumable(&bad_fee));

    let mut transfer_failed = sample_record(IcpRefillStatus::Failed);
    transfer_failed.error_code = Some(IcpRefillErrorCode::LedgerTransferFailed);
    assert!(!is_resumable(&transfer_failed));
    assert!(!is_resumable(&sample_record(IcpRefillStatus::Completed)));
}

#[test]
fn bad_fee_retry_requires_no_block_index() {
    let mut record = sample_record(IcpRefillStatus::Failed);
    record.error_code = Some(IcpRefillErrorCode::BadFee);
    assert!(can_retry_bad_fee(&record));

    record.ledger_block_index = Some(10);
    assert!(!can_retry_bad_fee(&record));

    record.ledger_block_index = None;
    record.error_code = Some(IcpRefillErrorCode::LedgerTransferFailed);
    assert!(!can_retry_bad_fee(&record));
}

#[test]
fn transfer_window_stale_applies_to_bad_fee_retry() {
    let mut record = sample_record(IcpRefillStatus::Failed);
    record.error_code = Some(IcpRefillErrorCode::BadFee);

    assert!(transfer_window_stale(
        &record,
        record.created_at_time_ns + TX_WINDOW_NANOS + 1
    ));
}

#[test]
fn retry_request_must_match_stored_operation_identity() {
    let record = sample_record(IcpRefillStatus::Requested);
    let mut request = request_for(&record);
    validate_retry_request_matches_record(&request, &record).expect("matching retry");

    request.amount_e8s += 1;
    let err = validate_retry_request_matches_record(&request, &record)
        .expect_err("changed amount must fail");
    assert!(err.to_string().contains("amount_e8s"));
}

#[test]
fn hub_self_refill_operation_id_binds_identity_amount_and_time() {
    let source = p(1);
    let target = p(3);
    let first = hub_self_refill_operation_id(source, None, target, 100, 1_000);
    let same = hub_self_refill_operation_id(source, None, target, 100, 1_000);
    let different_amount = hub_self_refill_operation_id(source, None, target, 101, 1_000);
    let different_time = hub_self_refill_operation_id(source, None, target, 100, 1_001);

    assert_eq!(first, same);
    assert_ne!(first, different_amount);
    assert_ne!(first, different_time);
}
