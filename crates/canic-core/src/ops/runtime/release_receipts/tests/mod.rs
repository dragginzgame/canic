//! Exact original authority, expiry-independent discovery and bounded metadata proofs.

use super::*;
use crate::{
    ids::IntentId,
    model::replay::{ReplayActor, ReplayCostGuardSettlement},
    storage::stable::replay::{ReplayReceiptRecord, ReplayReceiptStore},
};

fn fixture(id: u8) -> ReplayReceiptEntryRecord {
    let command = CommandKind::new("test.release.receipt.v1").unwrap();
    ReplayReceiptEntryRecord {
        key: ReplayReceiptOps::slot_key(&command, OperationId::from_bytes([id; 32])),
        record: ReplayReceiptRecord {
            schema_version: REPLAY_RECEIPT_SCHEMA_VERSION,
            command_kind: command.as_str().into(),
            operation_id: [id; 32],
            actor: ReplayActor {
                effective_principal: Principal::from_slice(&[2]),
                auth_kind: AuthKind::DelegatedToken,
            },
            payload_hash_schema_version: 1,
            payload_hash: [3; 32],
            status: ReplayReceiptStatus::ExternalEffectInFlight,
            created_at_ns: 1,
            updated_at_ns: 2,
            expires_at_ns: Some(3),
            response_schema_version: None,
            response_bytes: None,
            staged_response_schema_version: Some(1),
            staged_response_bytes: Some(vec![9; 1024 * 1024]),
            cost_guard_settlement: Some(ReplayCostGuardSettlement {
                quota_intent_id: IntentId(7),
                reservation_intent_id: IntentId(8),
            }),
            effect: Some(ExternalEffectDescriptor::ManagementCall {
                canister: Principal::from_slice(&[4]),
                method: "deposit_cycles".into(),
            }),
        },
    }
}

#[test]
fn census_retains_expired_uncertainty_and_history_without_pruning_or_mutation() {
    ReplayReceiptStore::reset_for_tests();
    let owner = Principal::from_slice(&[1]);
    assert_eq!(observe(owner, None).unwrap().entry, None);
    for (id, status) in [
        (1, ReplayReceiptStatus::Reserved),
        (2, ReplayReceiptStatus::ExternalEffectInFlight),
        (
            3,
            ReplayReceiptStatus::RecoveryRequired {
                reason: RecoveryReason::CostSettlementFailed,
            },
        ),
        (4, ReplayReceiptStatus::Committed),
    ] {
        let mut entry = fixture(id);
        entry.record.status = status;
        ReplayReceiptStore::upsert(entry.key, entry.record);
    }
    let before = ReplayReceiptStore::export();
    let mut cursor = None;
    let mut seen = Vec::new();
    for expected in &before.entries {
        let page = observe(owner, cursor).unwrap();
        assert_eq!(page.owner, owner);
        assert_eq!(observe(owner, cursor).unwrap(), page);
        let entry = page.entry.as_ref().unwrap();
        assert_eq!(entry.slot, expected.key.0);
        assert_eq!(entry.operation_id, expected.record.operation_id);
        assert_eq!(entry.actor, expected.record.actor.effective_principal);
        assert_eq!(entry.authentication, Authentication::DelegatedToken);
        assert_eq!(entry.expires_at_ns, Some(3));
        assert_eq!(
            entry.cost_guard_settlement,
            Some(ReplayReleaseSettlement {
                quota_intent_id: 7,
                reservation_intent_id: 8
            })
        );
        assert_eq!(
            entry.effect,
            Some(Effect::ManagementCall {
                canister: Principal::from_slice(&[4]),
                method: "deposit_cycles".into()
            })
        );
        let bytes = candid::encode_one(&page).unwrap();
        assert!(
            bytes.len() < 4096,
            "cached response bytes must stay outside the census"
        );
        assert_eq!(
            candid::decode_one::<ReplayReleaseResponse>(&bytes).unwrap(),
            page
        );
        seen.push((entry.operation_id, entry.phase));
        cursor = page.next_after;
    }
    assert_eq!(cursor, None);
    seen.sort_by_key(|(id, _)| *id);
    assert_eq!(
        seen,
        vec![
            ([1; 32], Phase::Reserved),
            ([2; 32], Phase::ExternalEffectInFlight),
            (
                [3; 32],
                Phase::RecoveryRequired(Reason::CostSettlementFailed)
            ),
            ([4; 32], Phase::Committed),
        ]
    );
    assert_eq!(ReplayReceiptStore::export(), before);
    assert_eq!(observe(owner, Some([255; 32])).unwrap().entry, None);
}

#[test]
fn refuses_mismatched_slot_schema_and_oversized_identity_without_changing_the_row() {
    ReplayReceiptStore::reset_for_tests();
    let mut wrong_slot = fixture(1);
    wrong_slot.key.0[0] ^= 1;
    let mut wrong_schema = fixture(2);
    wrong_schema.record.schema_version += 1;
    let mut oversized = fixture(3);
    oversized.record.command_kind = "x".repeat(MAXIMUM_IDENTITY_BYTES + 1);
    for (entry, code) in [
        (wrong_slot, InternalError::conflict().code()),
        (wrong_schema, InternalError::invariant().code()),
        (oversized, InternalError::resource_exhausted().code()),
    ] {
        ReplayReceiptStore::reset_for_tests();
        ReplayReceiptStore::upsert(entry.key, entry.record);
        let before = ReplayReceiptStore::export();
        assert_eq!(
            observe(Principal::from_slice(&[1]), None)
                .unwrap_err()
                .code(),
            code
        );
        assert_eq!(ReplayReceiptStore::export(), before);
    }
}

#[test]
fn retains_original_effect_identity_across_distinct_accounting_recovery_reasons() {
    let mut entry = fixture(1);
    entry.record.status = ReplayReceiptStatus::RecoveryRequired {
        reason: RecoveryReason::ResponseCommitFailed,
    };
    entry.record.effect = Some(ExternalEffectDescriptor::IcpTransfer {
        operation_id: OperationId::from_bytes([9; 32]),
    });
    let observed = project(entry.clone()).unwrap();
    assert_eq!(
        observed.phase,
        Phase::RecoveryRequired(Reason::ResponseCommitFailed)
    );
    assert_eq!(
        observed.effect,
        Some(Effect::IcpTransfer {
            operation_id: [9; 32]
        })
    );
    entry.record.status = ReplayReceiptStatus::RecoveryRequired {
        reason: RecoveryReason::ExternalEffectStatusUnknown,
    };
    assert_eq!(
        project(entry.clone()).unwrap().phase,
        Phase::RecoveryRequired(Reason::ExternalEffectStatusUnknown)
    );
    entry.record.status = ReplayReceiptStatus::RecoveryRequired {
        reason: RecoveryReason::ComponentChildLifecycleInterrupted,
    };
    entry.record.actor.auth_kind = AuthKind::RoleAttestation;
    entry.record.effect = Some(ExternalEffectDescriptor::RootCanisterProvision {
        command_kind: CommandKind::new("child.allocate.v1").unwrap(),
    });
    let observed = project(entry).unwrap();
    assert_eq!(observed.authentication, Authentication::RoleAttestation);
    assert_eq!(
        observed.phase,
        Phase::RecoveryRequired(Reason::ComponentChildLifecycleInterrupted)
    );
    assert_eq!(
        observed.effect,
        Some(Effect::RootCanisterProvision {
            command_kind: "child.allocate.v1".into()
        })
    );
}
