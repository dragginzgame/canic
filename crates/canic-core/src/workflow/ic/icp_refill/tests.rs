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

fn stored_record(id: u64, operation_byte: u8, status: IcpRefillStatus) -> IcpRefillRecord {
    let mut record = sample_record(status);
    record.id = id;
    record.operation_id = [operation_byte; 32];
    IcpRefillRecordOps::insert(record.clone());
    record
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

#[test]
fn fifth_notify_processing_attempt_is_terminal() {
    let mut record = stored_record(10_001, 101, IcpRefillStatus::NotifyProcessing);
    record.ledger_block_index = Some(42);
    record.notify_attempts = MAX_NOTIFY_ATTEMPTS - 1;
    IcpRefillRecordOps::insert(record.clone());

    let record =
        IcpRefillRecordOps::mark_notify_attempt_started(record.id, record.updated_at_ns + 1)
            .expect("notify attempt should start");
    assert_eq!(record.notify_attempts, MAX_NOTIFY_ATTEMPTS);

    let record = mark_notify_processing(record.id, record.notify_attempts)
        .expect("fifth processing attempt should be terminal");
    assert_eq!(record.status, IcpRefillStatus::Failed);
    assert_eq!(
        record.error_code,
        Some(IcpRefillErrorCode::NotifyMaxAttempts)
    );
    assert_eq!(
        record.error_message.as_deref(),
        Some("notify_top_up returned Processing after max attempts")
    );
}

#[test]
fn fifth_notify_failure_attempt_is_terminal() {
    let mut record = stored_record(10_002, 102, IcpRefillStatus::Failed);
    record.ledger_block_index = Some(43);
    record.notify_attempts = MAX_NOTIFY_ATTEMPTS - 1;
    record.error_code = Some(IcpRefillErrorCode::NotifyFailed);
    IcpRefillRecordOps::insert(record.clone());

    let record =
        IcpRefillRecordOps::mark_notify_attempt_started(record.id, record.updated_at_ns + 1)
            .expect("notify attempt should start");
    assert_eq!(record.notify_attempts, MAX_NOTIFY_ATTEMPTS);

    let record = mark_retryable_notify_failure(
        record.id,
        record.notify_attempts,
        "notify_top_up transport error".to_string(),
    )
    .expect("fifth notify failure should be terminal");
    assert_eq!(record.status, IcpRefillStatus::Failed);
    assert_eq!(
        record.error_code,
        Some(IcpRefillErrorCode::NotifyMaxAttempts)
    );
    assert_eq!(
        record.error_message.as_deref(),
        Some("notify_top_up transport error")
    );
}

#[test]
fn notify_processing_before_attempt_cap_stays_retryable() {
    let mut record = stored_record(10_003, 103, IcpRefillStatus::Transferred);
    record.ledger_block_index = Some(44);
    IcpRefillRecordOps::insert(record.clone());

    let record = apply_notify_error(record.id, 1, NotifyTopUpError::Processing)
        .expect("processing should remain retryable before cap");

    assert_eq!(record.status, IcpRefillStatus::NotifyProcessing);
    assert_eq!(record.error_code, Some(IcpRefillErrorCode::Processing));
    assert!(should_notify(&record));
}

#[test]
fn notify_refunded_preserves_refund_block_index() {
    let record = stored_record(10_004, 104, IcpRefillStatus::Transferred);

    let record = apply_notify_error(
        record.id,
        1,
        NotifyTopUpError::Refunded {
            block_index: Some(55),
            reason: "refunded by cmc".to_string(),
        },
    )
    .expect("refunded notify result should be recorded");

    assert_eq!(record.status, IcpRefillStatus::Refunded);
    assert_eq!(record.error_code, Some(IcpRefillErrorCode::Refunded));
    assert_eq!(record.refund_block_index, Some(55));
    assert_eq!(record.error_message.as_deref(), Some("refunded by cmc"));
    assert!(!is_resumable(&record));
}

#[test]
fn notify_transaction_too_old_preserves_min_block_index() {
    let record = stored_record(10_005, 105, IcpRefillStatus::Transferred);

    let record = apply_notify_error(record.id, 1, NotifyTopUpError::TransactionTooOld(56))
        .expect("transaction-too-old notify result should be recorded");

    assert_eq!(record.status, IcpRefillStatus::TransactionTooOld);
    assert_eq!(
        record.error_code,
        Some(IcpRefillErrorCode::TransactionTooOld)
    );
    assert_eq!(record.transaction_too_old_min_block_index, Some(56));
    assert!(!is_resumable(&record));
}

#[test]
fn notify_invalid_transaction_is_terminal() {
    let record = stored_record(10_006, 106, IcpRefillStatus::Transferred);

    let record = apply_notify_error(
        record.id,
        1,
        NotifyTopUpError::InvalidTransaction("bad top-up block".to_string()),
    )
    .expect("invalid transaction notify result should be recorded");

    assert_eq!(record.status, IcpRefillStatus::InvalidTransaction);
    assert_eq!(
        record.error_code,
        Some(IcpRefillErrorCode::InvalidTransaction)
    );
    assert_eq!(record.error_message.as_deref(), Some("bad top-up block"));
    assert!(!is_resumable(&record));
}

#[test]
fn notify_other_error_stays_retryable_before_attempt_cap() {
    let mut record = stored_record(10_007, 107, IcpRefillStatus::Transferred);
    record.ledger_block_index = Some(57);
    IcpRefillRecordOps::insert(record.clone());

    let record = apply_notify_error(
        record.id,
        1,
        NotifyTopUpError::Other {
            error_code: 12,
            error_message: "cmc busy".to_string(),
        },
    )
    .expect("other notify error should remain retryable before cap");

    assert_eq!(record.status, IcpRefillStatus::Failed);
    assert_eq!(record.error_code, Some(IcpRefillErrorCode::NotifyFailed));
    assert_eq!(
        record.error_message.as_deref(),
        Some("notify_top_up error 12: cmc busy")
    );
    assert!(can_retry_notify(&record));
}

#[test]
fn transfer_bad_fee_updates_persisted_fee() {
    let record = stored_record(10_008, 108, IcpRefillStatus::Requested);

    let record = apply_transfer_error(
        record.id,
        TransferError::BadFee {
            expected_fee: Nat::from(20_000_u64),
        },
    )
    .expect("bad fee should update persisted fee");

    assert_eq!(record.status, IcpRefillStatus::Failed);
    assert_eq!(record.error_code, Some(IcpRefillErrorCode::BadFee));
    assert_eq!(record.fee_e8s, 20_000);
    assert!(can_retry_bad_fee(&record));
}

#[test]
fn transfer_duplicate_records_recovered_block_index() {
    let record = stored_record(10_009, 109, IcpRefillStatus::Requested);

    let record = apply_transfer_error(
        record.id,
        TransferError::Duplicate {
            duplicate_of: Nat::from(58_u64),
        },
    )
    .expect("duplicate transfer should recover block index");

    assert_eq!(record.status, IcpRefillStatus::Transferred);
    assert_eq!(record.error_code, Some(IcpRefillErrorCode::Duplicate));
    assert_eq!(record.ledger_block_index, Some(58));
    assert!(should_notify(&record));
}

#[test]
fn transfer_too_old_marks_retry_window_stale() {
    let record = stored_record(10_010, 110, IcpRefillStatus::Requested);

    let record = apply_transfer_error(record.id, TransferError::TooOld)
        .expect("too-old transfer should mark stale retry window");

    assert_eq!(record.status, IcpRefillStatus::Failed);
    assert_eq!(
        record.error_code,
        Some(IcpRefillErrorCode::TransferWindowStale)
    );
    assert!(!is_resumable(&record));
}
