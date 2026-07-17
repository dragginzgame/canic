use super::{cost_guard::*, execution::*, hub::*, manual::*, replay::*, *};
use crate::{
    InternalErrorClass,
    cdk::{
        candid::Nat,
        types::{Principal, TC},
    },
    config::schema::CanisterKind,
    domain::icp_refill::{IcpRefillErrorCode, IcpRefillMode, IcpRefillStatus},
    dto::error::ErrorCode,
    ids::{CanisterRole, SubnetRole},
    infra::ic::icp_refill::{NotifyTopUpError, TransferError},
    model::replay::{ExternalEffectDescriptor, OperationId, RecoveryReason, ReplayReceiptStatus},
    ops::{
        cost_guard::CostGuardOps,
        replay::receipt::{record_cost_guard_settlement, stage_receipt_response},
        storage::icp_refill::{
            IcpRefillRecordCreateInput, IcpRefillRecordOps, IcpRefillRecordOpsError,
            IcpRefillStoreOps,
        },
        storage::replay::ReplayReceiptOps,
    },
    replay_policy::CostClass,
    storage::stable::icp_refill::IcpRefillRecord,
    test::{config::ConfigTestBuilder, seams::lock, support::import_test_env},
    view::icp_refill::IcpRefillOperation,
};
use std::str::FromStr;

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
        status: status.into(),
        error_code: None,
        error_message: None,
        refund_block_index: None,
        transaction_too_old_min_block_index: None,
        created_at_ns: 1_000,
        updated_at_ns: 1_000,
    }
}

fn operation_from_record(record: &IcpRefillRecord) -> IcpRefillOperation {
    IcpRefillOperation {
        id: record.id,
        operation_id: record.operation_id,
        source_canister: record.source_canister,
        source_subaccount: record.source_subaccount,
        target_canister: record.target_canister,
        ledger_canister_id: record.ledger_canister_id,
        cmc_canister_id: record.cmc_canister_id,
        amount_e8s: record.amount_e8s,
        fee_e8s: record.fee_e8s,
        memo: record.memo.clone(),
        created_at_time_ns: record.created_at_time_ns,
        ledger_block_index: record.ledger_block_index,
        notify_attempts: record.notify_attempts,
        cycles_sent: record.cycles_sent.clone(),
        status: record.status.into(),
        error_code: record.error_code.map(Into::into),
        error_message: record.error_message.clone(),
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

fn request_with_operation(operation_byte: u8) -> IcpRefillRequest {
    let mut record = sample_record(IcpRefillStatus::Requested);
    record.operation_id = [operation_byte; 32];
    request_for(&record)
}

fn stored_record(id: u64, operation_byte: u8, status: IcpRefillStatus) -> IcpRefillRecord {
    let mut record = sample_record(status);
    record.id = id;
    record.operation_id = [operation_byte; 32];
    IcpRefillRecordOps::insert(record.clone()).expect("insert refill record");
    record
}

fn create_input(operation_byte: u8) -> IcpRefillRecordCreateInput {
    IcpRefillRecordCreateInput {
        operation_id: [operation_byte; 32],
        source_canister: p(201),
        source_subaccount: Some([202; 32]),
        target_canister: p(203),
        ledger_canister_id: p(204),
        cmc_canister_id: p(205),
        cmc_to_account_owner: p(205),
        cmc_to_account_subaccount: Some([206; 32]),
        amount_e8s: 100_000_000,
        fee_e8s: 10_000,
        memo: IcpRefillOps::topup_memo(),
        created_at_time_ns: 1_000,
        now_ns: 1_000,
    }
}

#[test]
fn atomic_create_rejects_distinct_active_operation_for_same_refill_key() {
    let first = IcpRefillRecordOps::create_or_get(create_input(201)).expect("create first refill");
    let error = IcpRefillRecordOps::create_or_get(create_input(202))
        .expect_err("second active refill must conflict");

    assert!(matches!(
        error,
        IcpRefillRecordOpsError::ConcurrentOperation { id } if id == first.id
    ));
    assert!(
        IcpRefillRecordOps::find_by_operation_id([202; 32])
            .expect("operation index lookup")
            .is_none()
    );
}

#[test]
fn atomic_create_allows_next_operation_after_terminal_outcome() {
    let first = IcpRefillRecordOps::create_or_get(create_input(203)).expect("create first refill");
    IcpRefillRecordOps::mark_completed(first.id, Nat::from(1_u64), 2_000)
        .expect("complete first refill");

    let second = IcpRefillRecordOps::create_or_get(create_input(204))
        .expect("create refill after terminal outcome");

    assert_ne!(first.id, second.id);
}

#[test]
fn atomic_create_rejects_distinct_operation_while_failure_is_retryable() {
    let first = IcpRefillRecordOps::create_or_get(create_input(205)).expect("create first refill");
    IcpRefillRecordOps::mark_bad_fee(first.id, 20_000, "fee changed".to_string(), 2_000)
        .expect("record retryable bad fee");

    let error = IcpRefillRecordOps::create_or_get(create_input(206))
        .expect_err("retryable refill must remain active");

    assert!(matches!(
        error,
        IcpRefillRecordOpsError::ConcurrentOperation { id } if id == first.id
    ));
}

#[test]
fn missing_build_network_fails_closed() {
    let error = require_build_network(None).expect_err("network must be explicit");

    assert_eq!(error.class(), InternalErrorClass::Invariant);
    assert_eq!(error.origin(), InternalErrorOrigin::Workflow);
    assert_eq!(
        require_build_network(Some(BuildNetwork::Ic)).expect("explicit network"),
        BuildNetwork::Ic
    );
}

#[test]
fn missing_direct_child_funding_policy_preserves_config_failure() {
    let _guard = lock();
    let target = p(212);
    let _config = ConfigTestBuilder::new()
        .with_prime_canister_kind("funding_hub", CanisterKind::Service)
        .install();
    import_test_env("funding_hub", SubnetRole::PRIME, p(210));

    let error = configured_funding_cooldown_retry_after_secs(
        &CanisterRole::new("missing_child"),
        target,
        1_000,
    )
    .expect_err("missing child config must fail");

    assert_eq!(error.class(), InternalErrorClass::Ops);
    assert_eq!(error.origin(), InternalErrorOrigin::Ops);
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
    validate_ledger_decimals(6).expect_err("non-ICP decimals must fail");
}

#[test]
fn refill_canister_overrides_follow_config_resolution_fields() {
    assert_eq!(
        refill_canister_overrides(None),
        IcpRefillCanisterOverrides::default()
    );

    let policy = IcpRefillPolicy {
        enabled: true,
        min_hub_cycles_before_refill: Cycles::new(2 * TC),
        max_refill_e8s_per_call: 100_000_000,
        min_xdr_permyriad_per_icp: None,
        ledger_canister_id: Some(p(11)),
        cmc_canister_id: Some(p(12)),
        allow_ic_system_canister_overrides: true,
    };

    assert_eq!(
        refill_canister_overrides(Some(&policy)),
        IcpRefillCanisterOverrides {
            ledger_canister_id: Some(p(11)),
            cmc_canister_id: Some(p(12)),
            allow_ic_overrides: true,
        }
    );
}

#[test]
fn transfer_window_stale_uses_strict_tx_window() {
    let record = sample_record(IcpRefillStatus::Requested);

    assert!(!IcpRefillStoreOps::transfer_window_stale(
        &operation_from_record(&record),
        record.created_at_time_ns + TX_WINDOW_NANOS,
        TX_WINDOW_NANOS
    ));
    assert!(IcpRefillStoreOps::transfer_window_stale(
        &operation_from_record(&record),
        record.created_at_time_ns + TX_WINDOW_NANOS + 1,
        TX_WINDOW_NANOS
    ));
}

#[test]
fn transfer_window_stale_requires_requested_without_block_index() {
    let mut record = sample_record(IcpRefillStatus::Requested);
    record.ledger_block_index = Some(10);
    assert!(!IcpRefillStoreOps::transfer_window_stale(
        &operation_from_record(&record),
        record.created_at_time_ns + TX_WINDOW_NANOS + 1,
        TX_WINDOW_NANOS
    ));

    let record = sample_record(IcpRefillStatus::Failed);
    assert!(!IcpRefillStoreOps::transfer_window_stale(
        &operation_from_record(&record),
        record.created_at_time_ns + TX_WINDOW_NANOS + 1,
        TX_WINDOW_NANOS
    ));
}

#[test]
fn notify_retry_only_allows_notify_failed_with_block_index() {
    let mut record = sample_record(IcpRefillStatus::Failed);
    record.error_code = Some(IcpRefillErrorCode::NotifyFailed.into());
    record.ledger_block_index = Some(10);
    assert!(IcpRefillStoreOps::can_retry_notify(&operation_from_record(
        &record
    )));
    assert!(IcpRefillStoreOps::should_notify(&operation_from_record(
        &record
    )));

    record.ledger_block_index = None;
    assert!(!IcpRefillStoreOps::can_retry_notify(
        &operation_from_record(&record)
    ));
    assert!(!IcpRefillStoreOps::should_notify(&operation_from_record(
        &record
    )));

    let mut transferred = sample_record(IcpRefillStatus::Transferred);
    transferred.ledger_block_index = Some(11);
    assert!(IcpRefillStoreOps::should_notify(&operation_from_record(
        &transferred
    )));
}

#[test]
fn hub_self_refill_resumes_in_flight_and_retryable_records() {
    assert!(IcpRefillRecordOps::is_resumable(&sample_record(
        IcpRefillStatus::Requested
    )));
    assert!(IcpRefillRecordOps::is_resumable(&sample_record(
        IcpRefillStatus::Transferred
    )));
    assert!(IcpRefillRecordOps::is_resumable(&sample_record(
        IcpRefillStatus::NotifyProcessing
    )));

    let mut notify_failed = sample_record(IcpRefillStatus::Failed);
    notify_failed.error_code = Some(IcpRefillErrorCode::NotifyFailed.into());
    notify_failed.ledger_block_index = Some(11);
    assert!(IcpRefillRecordOps::is_resumable(&notify_failed));

    let mut bad_fee = sample_record(IcpRefillStatus::Failed);
    bad_fee.error_code = Some(IcpRefillErrorCode::BadFee.into());
    assert!(IcpRefillRecordOps::is_resumable(&bad_fee));

    let mut transfer_failed = sample_record(IcpRefillStatus::Failed);
    transfer_failed.error_code = Some(IcpRefillErrorCode::LedgerTransferFailed.into());
    assert!(!IcpRefillRecordOps::is_resumable(&transfer_failed));
    assert!(!IcpRefillRecordOps::is_resumable(&sample_record(
        IcpRefillStatus::Completed
    )));
}

#[test]
fn bad_fee_retry_requires_no_block_index() {
    let mut record = sample_record(IcpRefillStatus::Failed);
    record.error_code = Some(IcpRefillErrorCode::BadFee.into());
    assert!(IcpRefillStoreOps::can_retry_bad_fee(
        &operation_from_record(&record)
    ));

    record.ledger_block_index = Some(10);
    assert!(!IcpRefillStoreOps::can_retry_bad_fee(
        &operation_from_record(&record)
    ));

    record.ledger_block_index = None;
    record.error_code = Some(IcpRefillErrorCode::LedgerTransferFailed.into());
    assert!(!IcpRefillStoreOps::can_retry_bad_fee(
        &operation_from_record(&record)
    ));
}

#[test]
fn transfer_window_stale_applies_to_bad_fee_retry() {
    let mut record = sample_record(IcpRefillStatus::Failed);
    record.error_code = Some(IcpRefillErrorCode::BadFee.into());

    assert!(IcpRefillStoreOps::transfer_window_stale(
        &operation_from_record(&record),
        record.created_at_time_ns + TX_WINDOW_NANOS + 1,
        TX_WINDOW_NANOS
    ));
}

#[test]
fn retry_request_must_match_stored_operation_identity() {
    let record = sample_record(IcpRefillStatus::Requested);
    let mut request = request_for(&record);
    IcpRefillStoreOps::validate_retry_request_matches_operation(
        &request,
        &operation_from_record(&record),
    )
    .expect("matching retry");

    request.amount_e8s += 1;
    IcpRefillStoreOps::validate_retry_request_matches_operation(
        &request,
        &operation_from_record(&record),
    )
    .expect_err("changed amount must fail");
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
fn refill_replay_operation_id_uses_request_bytes_exactly() {
    let request = request_with_operation(77);

    assert_eq!(
        icp_refill_operation_id(&request).into_bytes(),
        request.operation_id
    );
}

#[test]
fn refill_replay_reserve_input_carries_shared_identity() {
    let request = request_with_operation(78);
    let caller = p(90);
    let now_ns = 123_456;
    let input = icp_refill_replay_reserve_input(&request, caller, now_ns);

    assert_eq!(input.command_kind.as_str(), "icp.refill.v1");
    assert_eq!(input.operation_id.into_bytes(), request.operation_id);
    assert_eq!(input.actor, icp_refill_replay_actor(caller));
    assert_eq!(input.now_ns, now_ns);
    assert_eq!(
        input.payload_hash,
        icp_refill_payload_hash(&input.command_kind, &input.actor, &request)
    );
}

#[test]
fn refill_replay_payload_hash_excludes_operation_id() {
    let command_kind = icp_refill_command_kind();
    let actor = icp_refill_replay_actor(p(90));
    let first = request_with_operation(1);
    let second = request_with_operation(2);

    assert_eq!(
        icp_refill_payload_hash(&command_kind, &actor, &first),
        icp_refill_payload_hash(&command_kind, &actor, &second)
    );
}

#[test]
fn refill_replay_payload_hash_binds_actor_and_transfer_fields() {
    let command_kind = icp_refill_command_kind();
    let actor = icp_refill_replay_actor(p(90));
    let other_actor = icp_refill_replay_actor(p(91));
    let request = request_with_operation(3);

    let base = icp_refill_payload_hash(&command_kind, &actor, &request);
    assert_ne!(
        base,
        icp_refill_payload_hash(&command_kind, &other_actor, &request)
    );

    let mut changed_amount = request.clone();
    changed_amount.amount_e8s += 1;
    assert_ne!(
        base,
        icp_refill_payload_hash(&command_kind, &actor, &changed_amount)
    );

    let mut changed_subaccount = request.clone();
    changed_subaccount.source_subaccount = Some([9; 32]);
    assert_ne!(
        base,
        icp_refill_payload_hash(&command_kind, &actor, &changed_subaccount)
    );

    let mut changed_target = request;
    changed_target.target_canister = p(92);
    assert_ne!(
        base,
        icp_refill_payload_hash(&command_kind, &actor, &changed_target)
    );
}

#[test]
fn refill_replay_commits_terminal_response_for_replay() {
    let request = request_with_operation(180);
    let input = icp_refill_replay_reserve_input(&request, p(90), 1_000);
    let IcpRefillReplayReservation::Fresh {
        operation_id,
        token,
    } = reserve_icp_refill_replay(input).expect("fresh reservation")
    else {
        panic!("expected fresh reservation");
    };

    let mut record = sample_record(IcpRefillStatus::Completed);
    record.operation_id = operation_id;
    record.ledger_block_index = Some(123);
    record.cycles_sent = Some(Nat::from(456_u64));
    let operation = operation_from_record(&record);
    let response = IcpRefillStoreOps::to_response(&operation);
    finish_icp_refill_replay(&token, &operation, &response, None)
        .expect("commit terminal response");

    let replay = reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_001))
        .expect("committed replay");
    let IcpRefillReplayReservation::Replay(cached) = replay else {
        panic!("expected cached replay");
    };
    assert_eq!(cached.operation_id, response.operation_id);
    assert_eq!(cached.status, IcpRefillStatus::Completed);
    assert_eq!(cached.ledger_block_index, Some(123));
    assert_eq!(cached.cycles_sent, Some(Nat::from(456_u64)));
}

#[test]
fn refill_replay_does_not_commit_when_cost_guard_completion_fails() {
    CostGuardOps::reset_for_tests();
    let request = request_with_operation(190);
    let IcpRefillReplayReservation::Fresh {
        operation_id,
        token,
    } = reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_000))
        .expect("fresh reservation")
    else {
        panic!("expected fresh reservation");
    };
    let mut record = sample_record(IcpRefillStatus::Completed);
    record.operation_id = operation_id;
    let operation = operation_from_record(&record);
    let response = IcpRefillStoreOps::to_response(&operation);
    mark_icp_refill_transfer_effect(&token, &operation).expect("mark transfer effect");
    let permit = CostGuardOps::reserve(icp_refill_cost_guard_request(
        &token,
        p(99),
        10_000_000_000,
        10,
    ))
    .expect("cost permit");
    record_cost_guard_settlement(&token, permit.replay_settlement(), 1_000)
        .expect("record settlement identity");
    CostGuardOps::abort(&permit).expect("invalidate cost permit");

    finish_icp_refill_replay(&token, &operation, &response, Some(&permit))
        .expect_err("cost settlement failure must reject replay commit");

    let receipt = ReplayReceiptOps::get(token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(
        receipt.status,
        ReplayReceiptStatus::RecoveryRequired {
            reason: RecoveryReason::CostSettlementFailed
        }
    );
    assert!(receipt.response_bytes.is_none());
    assert!(receipt.staged_response_bytes.is_some());
}

#[test]
fn refill_retry_finishes_cost_settlement_without_ledger_call() {
    CostGuardOps::reset_for_tests();
    ReplayReceiptOps::reset_for_tests();
    let request = request_with_operation(192);
    let IcpRefillReplayReservation::Fresh {
        operation_id,
        token,
    } = reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_000))
        .expect("fresh reservation")
    else {
        panic!("expected fresh reservation");
    };
    let permit = CostGuardOps::reserve(icp_refill_cost_guard_request(
        &token,
        p(99),
        10_000_000_000,
        crate::ops::ic::IcOps::now_secs(),
    ))
    .expect("cost permit");
    record_cost_guard_settlement(&token, permit.replay_settlement(), 1_000)
        .expect("record settlement identity");
    let mut record = sample_record(IcpRefillStatus::Completed);
    record.operation_id = operation_id;
    let operation = operation_from_record(&record);
    let response = IcpRefillStoreOps::to_response(&operation);
    mark_icp_refill_transfer_effect(&token, &operation).expect("mark transfer effect");
    stage_receipt_response(
        &token,
        crate::ops::replay::ICP_REFILL_REPLAY_RESPONSE_SCHEMA_VERSION,
        crate::ops::replay::encode_icp_refill_replay_response(&response).expect("response bytes"),
        1_001,
    )
    .expect("stage response");
    crate::ops::replay::receipt::mark_recovery_required(
        &token,
        RecoveryReason::CostSettlementFailed,
        1_002,
    )
    .expect("mark recovery");

    let replay = reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_003))
        .expect("retry should finish accounting");
    assert!(matches!(
        replay,
        IcpRefillReplayReservation::Replay(cached)
            if cached.operation_id == response.operation_id
                && cached.status == IcpRefillStatus::Completed
    ));
    let receipt = ReplayReceiptOps::get(token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(receipt.status, ReplayReceiptStatus::Committed);

    ReplayReceiptOps::reset_for_tests();
    CostGuardOps::reset_for_tests();
}

#[test]
fn refill_replay_resumable_response_aborts_reserved_receipt() {
    let request = request_with_operation(181);
    let IcpRefillReplayReservation::Fresh { token, .. } =
        reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_000))
            .expect("fresh reservation")
    else {
        panic!("expected fresh reservation");
    };
    let record = sample_record(IcpRefillStatus::Requested);
    let operation = operation_from_record(&record);
    let response = IcpRefillStoreOps::to_response(&operation);

    finish_icp_refill_replay(&token, &operation, &response, None)
        .expect("abort resumable response");

    assert!(matches!(
        reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_001))
            .expect("fresh after abort"),
        IcpRefillReplayReservation::Fresh { .. }
    ));
}

#[test]
fn refill_replay_keeps_receipt_when_cost_guard_recovery_fails() {
    CostGuardOps::reset_for_tests();
    let request = request_with_operation(191);
    let IcpRefillReplayReservation::Fresh { token, .. } =
        reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_000))
            .expect("fresh reservation")
    else {
        panic!("expected fresh reservation");
    };
    let mut record = sample_record(IcpRefillStatus::Requested);
    record.operation_id = request.operation_id;
    let operation = operation_from_record(&record);
    let response = IcpRefillStoreOps::to_response(&operation);
    mark_icp_refill_transfer_effect(&token, &operation).expect("mark transfer effect");
    let permit = CostGuardOps::reserve(icp_refill_cost_guard_request(
        &token,
        p(99),
        10_000_000_000,
        10,
    ))
    .expect("cost permit");
    CostGuardOps::abort(&permit).expect("invalidate cost permit");

    finish_icp_refill_replay(&token, &operation, &response, Some(&permit))
        .expect_err("cost recovery failure must preserve replay receipt");

    let receipt = ReplayReceiptOps::get(token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(receipt.status, ReplayReceiptStatus::ExternalEffectInFlight);
}

#[test]
fn refill_replay_payload_mismatch_maps_to_conflict() {
    let request = request_with_operation(182);
    let _reservation =
        reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_000))
            .expect("fresh reservation");

    let mut changed = request;
    changed.amount_e8s += 1;
    let err = reserve_icp_refill_replay(icp_refill_replay_reserve_input(&changed, p(90), 1_001))
        .expect_err("payload mismatch must fail");
    let public = err.public_error().expect("public replay error");
    assert_eq!(public.code, ErrorCode::Conflict);
}

#[test]
fn refill_cost_guard_request_uses_value_transfer_policy() {
    let request = request_with_operation(187);
    let IcpRefillReplayReservation::Fresh { token, .. } =
        reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_000))
            .expect("fresh reservation")
    else {
        panic!("expected fresh reservation");
    };
    let guard_request = icp_refill_cost_guard_request(&token, p(99), 5_000_000_000, 123);

    assert_eq!(guard_request.cost_class, CostClass::ValueTransfer);
    assert_eq!(guard_request.command_kind.as_str(), "icp.refill.v1");
    assert_eq!(guard_request.quota_subject, p(90));
    assert_eq!(
        guard_request.quota_window_secs,
        ICP_REFILL_VALUE_TRANSFER_QUOTA_WINDOW_SECONDS
    );
    assert_eq!(
        guard_request.max_operations_per_window,
        MAX_ICP_REFILL_VALUE_TRANSFER_OPERATIONS_PER_WINDOW
    );
    assert_eq!(
        guard_request.cycle_reservation_cycles,
        ICP_REFILL_VALUE_TRANSFER_CYCLE_RESERVATION_CYCLES
    );
    assert_eq!(
        guard_request.min_cycles_after_reservation,
        MIN_ICP_REFILL_CYCLES_AFTER_RESERVATION
    );
}

#[test]
fn refill_external_effect_boundary_requires_value_transfer_cost_permit() {
    let request = request_with_operation(189);
    let IcpRefillReplayReservation::Fresh { token, .. } =
        reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(92), 1_000))
            .expect("fresh reservation")
    else {
        panic!("expected fresh reservation");
    };

    let missing = require_icp_refill_cost_permit(None).expect_err("missing permit rejects");
    assert_eq!(missing.class(), InternalErrorClass::Invariant);
    assert_eq!(missing.origin(), InternalErrorOrigin::Workflow);

    let permit = CostGuardOps::reserve(icp_refill_cost_guard_request(
        &token,
        p(99),
        10_000_000_000,
        10_000,
    ))
    .expect("reserve value-transfer permit");
    let cost_permit = Some(permit);

    assert!(require_icp_refill_cost_permit(cost_permit.as_ref()).is_ok());
    CostGuardOps::abort(cost_permit.as_ref().expect("permit")).expect("abort permit");
}

#[test]
fn refill_value_transfer_cost_guard_enforces_actor_quota() {
    let request = request_with_operation(188);
    let IcpRefillReplayReservation::Fresh { token, .. } =
        reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(91), 1_000))
            .expect("fresh reservation")
    else {
        panic!("expected fresh reservation");
    };
    let now = 9_000;
    let balance = 10_000_000_000;

    for _ in 0..MAX_ICP_REFILL_VALUE_TRANSFER_OPERATIONS_PER_WINDOW {
        let permit =
            CostGuardOps::reserve(icp_refill_cost_guard_request(&token, p(99), balance, now))
                .expect("quota reservation");
        CostGuardOps::complete(&permit, now).expect("complete quota reservation");
    }

    let err = CostGuardOps::reserve(icp_refill_cost_guard_request(&token, p(99), balance, now))
        .expect_err("same actor quota bucket exhausted");
    let err = crate::workflow::cost_guard::map_cost_guard_reserve_error(err);
    assert_eq!(
        err.public_error().expect("quota rejection is public").code,
        ErrorCode::ResourceExhausted
    );
}

#[test]
fn refill_replay_marks_ledger_transfer_effect() {
    let request = request_with_operation(183);
    let IcpRefillReplayReservation::Fresh { token, .. } =
        reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_000))
            .expect("fresh reservation")
    else {
        panic!("expected fresh reservation");
    };
    let mut record = sample_record(IcpRefillStatus::Requested);
    record.operation_id = request.operation_id;

    let operation = operation_from_record(&record);
    mark_icp_refill_transfer_effect(&token, &operation).expect("mark transfer effect");

    let receipt = ReplayReceiptOps::get(token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(receipt.status, ReplayReceiptStatus::ExternalEffectInFlight);
    assert_eq!(
        receipt.effect,
        Some(ExternalEffectDescriptor::IcpTransfer {
            operation_id: OperationId::from_bytes(record.operation_id)
        })
    );
}

#[test]
fn refill_replay_marks_cmc_notify_effect() {
    let request = request_with_operation(184);
    let IcpRefillReplayReservation::Fresh { token, .. } =
        reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_000))
            .expect("fresh reservation")
    else {
        panic!("expected fresh reservation");
    };
    let mut record = sample_record(IcpRefillStatus::Transferred);
    record.operation_id = request.operation_id;

    let operation = operation_from_record(&record);
    mark_icp_refill_notify_effect(&token, &operation).expect("mark notify effect");

    let receipt = ReplayReceiptOps::get(token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(receipt.status, ReplayReceiptStatus::ExternalEffectInFlight);
    assert_eq!(
        receipt.effect,
        Some(ExternalEffectDescriptor::ManagementCall {
            canister: record.cmc_canister_id,
            method: "notify_top_up".to_string()
        })
    );
}

#[test]
fn refill_replay_resumable_response_aborts_in_flight_receipt() {
    let request = request_with_operation(185);
    let IcpRefillReplayReservation::Fresh { token, .. } =
        reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_000))
            .expect("fresh reservation")
    else {
        panic!("expected fresh reservation");
    };
    let mut record = sample_record(IcpRefillStatus::Requested);
    record.operation_id = request.operation_id;
    let operation = operation_from_record(&record);
    let response = IcpRefillStoreOps::to_response(&operation);
    mark_icp_refill_transfer_effect(&token, &operation).expect("mark transfer effect");

    finish_icp_refill_replay(&token, &operation, &response, None).expect("abort in-flight receipt");

    assert!(ReplayReceiptOps::get(token.key()).is_none());
    assert!(matches!(
        reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_001))
            .expect("fresh after abort"),
        IcpRefillReplayReservation::Fresh { .. }
    ));
}

#[test]
fn refill_replay_recovery_required_preserves_effect_receipt() {
    let request = request_with_operation(186);
    let IcpRefillReplayReservation::Fresh { token, .. } =
        reserve_icp_refill_replay(icp_refill_replay_reserve_input(&request, p(90), 1_000))
            .expect("fresh reservation")
    else {
        panic!("expected fresh reservation");
    };
    let mut record = sample_record(IcpRefillStatus::Requested);
    record.operation_id = request.operation_id;
    let operation = operation_from_record(&record);
    mark_icp_refill_transfer_effect(&token, &operation).expect("mark transfer effect");

    mark_icp_refill_recovery_required(
        &token,
        &operation,
        "ledger_transfer",
        &InternalError::infra(InternalErrorOrigin::Infra, "call failed"),
    )
    .expect("mark recovery required");

    let receipt = ReplayReceiptOps::get(token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(
        receipt.status,
        ReplayReceiptStatus::RecoveryRequired {
            reason: RecoveryReason::ExternalEffectStatusUnknown
        }
    );
    assert_eq!(
        receipt.effect,
        Some(ExternalEffectDescriptor::IcpTransfer {
            operation_id: OperationId::from_bytes(record.operation_id)
        })
    );
}

#[test]
fn direct_child_refill_grant_records_matching_parent() {
    let record = sample_record(IcpRefillStatus::Completed);

    assert_eq!(
        direct_child_refill_grant(
            &operation_from_record(&record),
            &Nat::from(123_u64),
            Some(record.source_canister)
        ),
        Some((record.target_canister, 123))
    );
}

#[test]
fn direct_child_refill_grant_ignores_non_child_targets() {
    let record = sample_record(IcpRefillStatus::Completed);

    assert_eq!(
        direct_child_refill_grant(&operation_from_record(&record), &Nat::from(123_u64), None),
        None
    );
    assert_eq!(
        direct_child_refill_grant(
            &operation_from_record(&record),
            &Nat::from(123_u64),
            Some(p(9))
        ),
        None
    );
}

#[test]
fn direct_child_refill_grant_saturates_large_cycle_totals() {
    let record = sample_record(IcpRefillStatus::Completed);
    let too_large =
        Nat::from_str("340282366920938463463374607431768211456").expect("u128 max plus one");

    assert_eq!(
        direct_child_refill_grant(
            &operation_from_record(&record),
            &too_large,
            Some(record.source_canister)
        ),
        Some((record.target_canister, u128::MAX))
    );
}

#[test]
fn fifth_notify_processing_attempt_is_terminal() {
    let mut record = stored_record(10_001, 101, IcpRefillStatus::NotifyProcessing);
    record.ledger_block_index = Some(42);
    record.notify_attempts = MAX_NOTIFY_ATTEMPTS - 1;
    IcpRefillRecordOps::insert(record.clone()).expect("insert refill record");

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
    record.error_code = Some(IcpRefillErrorCode::NotifyFailed.into());
    IcpRefillRecordOps::insert(record.clone()).expect("insert refill record");

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
    IcpRefillRecordOps::insert(record.clone()).expect("insert refill record");

    let record = apply_notify_error(record.id, 1, NotifyTopUpError::Processing)
        .expect("processing should remain retryable before cap");

    assert_eq!(record.status, IcpRefillStatus::NotifyProcessing);
    assert_eq!(record.error_code, Some(IcpRefillErrorCode::Processing));
    assert!(IcpRefillStoreOps::should_notify(&record));
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
    let stored = IcpRefillRecordOps::get(record.id).expect("stored refunded record");
    assert_eq!(stored.refund_block_index, Some(55));
    assert_eq!(record.error_message.as_deref(), Some("refunded by cmc"));
    assert!(!IcpRefillStoreOps::is_resumable(&record));
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
    let stored = IcpRefillRecordOps::get(record.id).expect("stored transaction-too-old record");
    assert_eq!(stored.transaction_too_old_min_block_index, Some(56));
    assert!(!IcpRefillStoreOps::is_resumable(&record));
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
    assert!(!IcpRefillStoreOps::is_resumable(&record));
}

#[test]
fn notify_other_error_stays_retryable_before_attempt_cap() {
    let mut record = stored_record(10_007, 107, IcpRefillStatus::Transferred);
    record.ledger_block_index = Some(57);
    IcpRefillRecordOps::insert(record.clone()).expect("insert refill record");

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
    assert!(IcpRefillStoreOps::can_retry_notify(&record));
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
    assert!(IcpRefillStoreOps::can_retry_bad_fee(&record));
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
    assert!(IcpRefillStoreOps::should_notify(&record));
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
    assert!(!IcpRefillStoreOps::is_resumable(&record));
}
