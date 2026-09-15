use super::*;
use crate::{
    model::replay::{OperationId, RecoveryReason, ReplayReceiptStatus},
    ops::cost_guard::CostGuardRequest,
    replay_policy::CostClass,
};

fn p(value: u8) -> Principal {
    Principal::from_slice(&[value])
}

fn pending(child: Principal, id: u8) -> ReplayReceiptRecord {
    ReplayReceiptRecord {
        schema_version: 1,
        command_kind: CHILD_FUNDING_COMMAND_KIND.into(),
        operation_id: [id; 32],
        actor: ReplayActor::direct_caller(child),
        payload_hash_schema_version: 1,
        payload_hash: [3; 32],
        status: ReplayReceiptStatus::ExternalEffectInFlight,
        created_at_ns: 1,
        updated_at_ns: 1,
        expires_at_ns: Some(2),
        response_schema_version: None,
        response_bytes: None,
        staged_response_schema_version: None,
        staged_response_bytes: None,
        cost_guard_settlement: None,
        effect: Some(ExternalEffectDescriptor::ManagementCall {
            canister: child,
            method: "deposit_cycles".into(),
        }),
    }
}

fn retain(record: ReplayReceiptRecord) {
    let command = CommandKind::new(&record.command_kind).unwrap();
    let key = ReplayReceiptOps::slot_key(&command, OperationId::from_bytes(record.operation_id));
    ReplayReceiptOps::upsert(key, record);
}

fn reset() {
    CostGuardOps::reset_for_tests();
    ReplayReceiptOps::reset_for_tests();
    CyclesFundingLedgerOps::reset_for_tests();
}

#[test]
fn child_usage_preserves_charges_and_pending_reservations_across_replay_expiry() {
    reset();
    let parent = p(1);
    let child = p(2);
    let permit = CostGuardOps::reserve(CostGuardRequest {
        cost_class: CostClass::ValueTransfer,
        command_kind: CommandKind::new(CHILD_FUNDING_COMMAND_KIND).unwrap(),
        quota_subject: child,
        payer: parent,
        now_secs: 10,
        quota_window_secs: 60,
        max_operations_per_window: 60,
        current_cycle_balance: 1_000,
        cycle_reservation_cycles: 30,
        min_cycles_after_reservation: 1,
    })
    .unwrap();
    let mut record = pending(child, 1);
    record.cost_guard_settlement = Some(permit.replay_settlement());
    retain(record.clone());
    CyclesFundingLedgerOps::record_child_grant(child, 130, 10);
    let before = snapshot(parent, child, 11_000_000_000).unwrap();
    assert_eq!(before.accounted_cycles.to_u128(), 130);
    assert_eq!(before.pending_operations, 1);
    assert_eq!(before.reserved_cycles, Some(30.into()));
    assert_eq!(
        snapshot(p(9), child, 11_000_000_000)
            .unwrap()
            .reserved_cycles,
        None
    );
    assert_eq!(
        snapshot(parent, child, 3_610_000_000_000)
            .unwrap()
            .reserved_cycles,
        None
    );
    assert_eq!(snapshot(parent, child, 11_000_000_000).unwrap(), before);
    CostGuardOps::complete(&permit, 12).unwrap();
    record.status = ReplayReceiptStatus::RecoveryRequired {
        reason: RecoveryReason::ResponseCommitFailed,
    };
    retain(record.clone());
    let recovery = snapshot(parent, child, 13_000_000_000).unwrap();
    assert_eq!(recovery.pending_operations, 1);
    assert_eq!(recovery.reserved_cycles, Some(0.into()));
    assert_eq!(recovery.accounted_cycles, before.accounted_cycles);
    record.status = ReplayReceiptStatus::Committed;
    retain(record);
    let completed = snapshot(parent, child, 14_000_000_000).unwrap();
    assert_eq!(completed.pending_operations, 0);
    assert_eq!(completed.reserved_cycles, Some(0.into()));
    assert_eq!(completed.accounted_cycles, before.accounted_cycles);
}

#[test]
fn child_usage_does_not_assume_missing_evidence_or_other_children_are_unused() {
    reset();
    retain(pending(p(2), 1));
    retain(pending(p(3), 2));
    let observed = snapshot(p(1), p(2), 10).unwrap();
    assert_eq!(observed.pending_operations, 1);
    assert_eq!(observed.reserved_cycles, None);
    assert_eq!(snapshot(p(1), p(4), 10).unwrap().pending_operations, 0);
    let bytes = candid::encode_one(&observed).unwrap();
    assert_eq!(
        candid::decode_one::<ChildFundingUsage>(&bytes).unwrap(),
        observed
    );
}
