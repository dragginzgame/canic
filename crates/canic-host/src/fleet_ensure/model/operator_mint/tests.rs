//! Exact durable shape and full-width quantity encoding tests.

use super::{
    OperatorMintAuthority, OperatorMintIntentRecord, OperatorMintNotificationOutcomeRecord,
    OperatorMintReceiptRecord,
};
use candid::Principal;
use serde_json::json;

fn receipt() -> OperatorMintReceiptRecord {
    OperatorMintReceiptRecord {
        intent: OperatorMintIntentRecord {
            authority: OperatorMintAuthority {
                operation_id: [1; 32],
                plan_sha256: [2; 32],
                funding_review_sha256: [3; 32],
                network_identity_sha256: [4; 32],
                operator: Principal::from_slice(&[1, 1]),
                icp_ledger: Principal::from_slice(&[2, 1]),
                cmc: Principal::from_slice(&[3, 1]),
                cycles_ledger: Principal::from_slice(&[4, 1]),
            },
            amount_e8s: u64::MAX,
            transfer_fee_e8s: 0,
            created_at_time_ns: u64::MAX,
            deposit_memo: [5; 32],
        },
        icp_block_index: u64::MAX,
        deposit_block_index: u128::MAX,
        destination_owner: Principal::from_slice(&[1, 1]),
        destination_subaccount: None,
        deposit_memo: [5; 32],
        gross_minted_cycles: u128::MAX,
        deposit_fee_cycles: 1,
        net_credit_cycles: u128::MAX - 1,
    }
}

#[test]
fn operator_mint_records_preserve_full_width_amounts() {
    let receipt = receipt();
    let encoded = serde_json::to_value(&receipt).unwrap();
    assert_eq!(encoded["deposit_block_index"], json!(u128::MAX.to_string()));
    assert_eq!(encoded["gross_minted_cycles"], json!(u128::MAX.to_string()));
    assert_eq!(encoded["destination_subaccount"], json!(null));
    assert_eq!(
        serde_json::from_value::<OperatorMintReceiptRecord>(encoded).unwrap(),
        receipt
    );
}

#[test]
fn operator_mint_records_require_every_declared_field() {
    let encoded = serde_json::to_value(receipt()).unwrap();
    for path in ["", "/intent", "/intent/authority"] {
        for key in encoded.pointer(path).unwrap().as_object().unwrap().keys() {
            let mut changed = encoded.clone();
            changed
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(key);
            assert!(serde_json::from_value::<OperatorMintReceiptRecord>(changed).is_err());
        }
        let mut changed = encoded.clone();
        changed
            .pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unrecognized_authority".into(), json!(true));
        assert!(serde_json::from_value::<OperatorMintReceiptRecord>(changed).is_err());
    }
}

#[test]
fn operator_mint_refund_requires_explicit_receipt_availability() {
    let refund = OperatorMintNotificationOutcomeRecord::Refunded {
        refund_block_index: None,
        reason: "refund receipt unavailable".into(),
    };
    let mut encoded = serde_json::to_value(&refund).unwrap();
    assert_eq!(encoded["refund_block_index"], json!(null));
    assert_eq!(
        serde_json::from_value::<OperatorMintNotificationOutcomeRecord>(encoded.clone()).unwrap(),
        refund
    );
    encoded
        .as_object_mut()
        .unwrap()
        .remove("refund_block_index");
    assert!(serde_json::from_value::<OperatorMintNotificationOutcomeRecord>(encoded).is_err());
}

#[test]
fn operator_mint_notification_quantities_round_trip_without_truncation() {
    let minted = OperatorMintNotificationOutcomeRecord::Minted {
        deposit_block_index: u128::MAX,
        gross_minted_cycles: u128::MAX,
        historical_balance_cycles: u128::MAX,
    };
    let encoded = serde_json::to_string(&minted).unwrap();
    assert_eq!(
        serde_json::from_str::<OperatorMintNotificationOutcomeRecord>(&encoded).unwrap(),
        minted
    );
}
