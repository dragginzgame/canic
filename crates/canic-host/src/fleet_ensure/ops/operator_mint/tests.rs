//! Exact retry arguments and independently encoded Ledger/CMC reply boundaries.

use super::{
    OperatorMintWireError, account_identifier, decode_notification_reply, decode_transfer_reply,
    notification_argument, prepare_intent, transfer_argument, wire,
};
use crate::fleet_ensure::model::operator_mint::{
    OperatorMintAuthority, OperatorMintIntentRecord,
    OperatorMintNotificationOutcomeRecord as Notify, OperatorMintTransferOutcomeRecord as Transfer,
};
use candid::Principal;

// ICP Ledger transfer and CMC notification contracts reviewed for RF2; the
// literal contracts are independent of the Rust derives under test.
const CONTRACT: &str = r"
type Tokens = record { e8s : nat64 };
type Timestamp = record { timestamp_nanos : nat64 };
type Transfer = record {
  memo : nat64; amount : Tokens; fee : Tokens;
  from_subaccount : opt blob; to : blob; created_at_time : opt Timestamp;
};
type TransferReply = variant {
  Ok : nat64;
  Err : variant {
    BadFee : record { expected_fee : Tokens };
    InsufficientFunds : record { balance : Tokens };
    TxTooOld : record { allowed_window_nanos : nat64 };
    TxCreatedInFuture; TxDuplicate : record { duplicate_of : nat64 };
  };
};
type Notification = record {
  block_index : nat64; deposit_memo : opt blob; to_subaccount : opt blob;
};
type NotificationReply = variant {
  Ok : record { balance : nat; block_index : nat; minted : nat };
  Err : variant {
    Refunded : record { block_index : opt nat64; reason : text };
    InvalidTransaction : text;
    Other : record { error_code : nat64; error_message : text };
    Processing; TransactionTooOld : nat64;
  };
};
";

fn assert_wire_type<T: candid::CandidType>(name: &str) {
    let (mut env, _) = candid_parser::utils::CandidSource::Text(CONTRACT)
        .load()
        .unwrap();
    let expected = env.find_type(name).unwrap().clone();
    let mut rust = candid::types::internal::TypeContainer::new();
    let ty = rust.add::<T>();
    let actual = env.merge_type(rust.env, ty);
    candid::types::subtype::equal(
        &mut std::collections::HashSet::new(),
        &env,
        &expected,
        &actual,
    )
    .unwrap();
}

#[test]
fn operator_mint_wire_types_match_reviewed_upstream_contracts() {
    assert_wire_type::<wire::TransferArgs>("Transfer");
    assert_wire_type::<Result<u64, wire::TransferError>>("TransferReply");
    assert_wire_type::<wire::NotifyMintArgs>("Notification");
    assert_wire_type::<Result<wire::NotifyMintOk, wire::NotifyMintError>>("NotificationReply");
}

fn authority() -> OperatorMintAuthority {
    OperatorMintAuthority {
        operation_id: [1; 32],
        plan_sha256: [2; 32],
        funding_review_sha256: [3; 32],
        network_identity_sha256: [4; 32],
        operator: Principal::from_text("4bkt6-4aaaa-aaaaf-aaaiq-cai").unwrap(),
        icp_ledger: Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap(),
        cmc: Principal::from_text("rkp4c-7iaaa-aaaaa-aaaca-cai").unwrap(),
        cycles_ledger: Principal::from_slice(&[4, 1]),
    }
}

fn intent() -> OperatorMintIntentRecord {
    prepare_intent(authority(), 100_000_000, 10_000, 123_456).unwrap()
}

fn bytes(idl: &str) -> Vec<u8> {
    candid_parser::parse_idl_args(idl)
        .unwrap()
        .to_bytes()
        .unwrap()
}

fn account_bytes(hex: &str) -> [u8; 32] {
    std::array::from_fn(|index| u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap())
}

#[test]
fn operator_mint_transfer_matches_upstream_account_vector_and_exact_intent() {
    let intent = intent();
    let args: wire::TransferArgs =
        candid::decode_one(&transfer_argument(&intent).unwrap()).unwrap();
    // ic-ledger-types 0.16.0's principal_to_subaccount vector, independently published.
    assert_eq!(
        args.to,
        account_bytes("d8646d1cbe44002026fa3e0d86d51a560b1c31d669bc8b7f66421c1b2feaa59f")
    );
    assert_eq!(args.memo, 0x544e_494d);
    assert_eq!(args.amount.e8s, intent.amount_e8s);
    assert_eq!(args.fee.e8s, intent.transfer_fee_e8s);
    assert!(args.from_subaccount.is_none());
    assert_eq!(
        args.created_at_time.unwrap().timestamp_nanos,
        intent.created_at_time_ns
    );
    let args: wire::NotifyMintArgs =
        candid::decode_one(&notification_argument(&intent, 42).unwrap()).unwrap();
    assert_eq!(args.block_index, 42);
    assert_eq!(args.deposit_memo, Some(intent.deposit_memo.to_vec()));
    assert!(args.to_subaccount.is_none());
}

#[test]
fn operator_mint_account_checksum_matches_upstream_default_account_vector() {
    let owner =
        Principal::from_text("iooej-vlrze-c5tme-tn7qt-vqe7z-7bsj5-ebxlc-hlzgs-lueo3-3yast-pae")
            .unwrap();
    assert_eq!(
        account_identifier(owner, &[0; 32]),
        account_bytes("bdc4ee05d42cd0669786899f256c8fd7217fa71177bd1fa7b9534f568680a938")
    );
}

#[test]
fn operator_mint_serialized_restart_preserves_both_request_bytes() {
    let original = intent();
    let recovered: OperatorMintIntentRecord =
        serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();
    assert_eq!(
        transfer_argument(&original).unwrap(),
        transfer_argument(&recovered).unwrap()
    );
    assert_eq!(
        notification_argument(&original, u64::MAX).unwrap(),
        notification_argument(&recovered, u64::MAX).unwrap()
    );
    assert_ne!(
        notification_argument(&original, 1).unwrap(),
        notification_argument(&original, 2).unwrap()
    );
}

#[test]
fn operator_mint_memo_binds_every_reviewed_input() {
    let original = intent();
    let mutations: [fn(&mut OperatorMintIntentRecord); 12] = [
        |r| r.authority.operation_id[0] ^= 1,
        |r| r.authority.plan_sha256[0] ^= 1,
        |r| r.authority.funding_review_sha256[0] ^= 1,
        |r| r.authority.network_identity_sha256[0] ^= 1,
        |r| r.authority.operator = Principal::from_slice(&[9, 1]),
        |r| r.authority.icp_ledger = Principal::from_slice(&[9, 1]),
        |r| r.authority.cmc = Principal::from_slice(&[9, 1]),
        |r| r.authority.cycles_ledger = Principal::from_slice(&[9, 1]),
        |r| r.amount_e8s += 1,
        |r| r.transfer_fee_e8s += 1,
        |r| r.created_at_time_ns += 1,
        |r| r.deposit_memo[0] ^= 1,
    ];
    for mutate in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert!(matches!(
            transfer_argument(&changed),
            Err(OperatorMintWireError::MemoMismatch)
        ));
        assert!(matches!(
            notification_argument(&changed, 42),
            Err(OperatorMintWireError::MemoMismatch)
        ));
    }
}

#[test]
fn operator_mint_rejects_invalid_debits_and_non_signing_authority() {
    for (amount, fee, time) in [(0, 1, 1), (1, 1, 0), (u64::MAX, 1, 1)] {
        assert!(matches!(
            prepare_intent(authority(), amount, fee, time),
            Err(OperatorMintWireError::InvalidAmounts)
        ));
    }
    let mutations: [fn(&mut OperatorMintAuthority, Principal); 4] = [
        |a, p| a.operator = p,
        |a, p| a.icp_ledger = p,
        |a, p| a.cmc = p,
        |a, p| a.cycles_ledger = p,
    ];
    for principal in [Principal::anonymous(), Principal::management_canister()] {
        for mutate in mutations {
            let mut changed = authority();
            mutate(&mut changed, principal);
            assert!(matches!(
                prepare_intent(changed, 1, 1, 1),
                Err(OperatorMintWireError::InvalidAuthority)
            ));
        }
    }
}

#[test]
fn operator_mint_transfer_replies_preserve_receipts_and_retry_constraints() {
    for (idl, expected) in [
        (
            "(variant { Ok = 42 : nat64 })",
            Transfer::Accepted { block_index: 42 },
        ),
        (
            "(variant { Err = variant { TxDuplicate = record { duplicate_of = 42 : nat64 } } })",
            Transfer::Duplicate { block_index: 42 },
        ),
        (
            "(variant { Err = variant { BadFee = record { expected_fee = record { e8s = 9 : nat64 } } } })",
            Transfer::BadFee {
                expected_fee_e8s: 9,
            },
        ),
        (
            "(variant { Err = variant { InsufficientFunds = record { balance = record { e8s = 8 : nat64 } } } })",
            Transfer::InsufficientFunds { balance_e8s: 8 },
        ),
        (
            "(variant { Err = variant { TxTooOld = record { allowed_window_nanos = 7 : nat64 } } })",
            Transfer::TooOld {
                allowed_window_ns: 7,
            },
        ),
        (
            "(variant { Err = variant { TxCreatedInFuture } })",
            Transfer::CreatedInFuture,
        ),
    ] {
        assert_eq!(decode_transfer_reply(&bytes(idl)).unwrap(), expected);
    }
}

#[test]
fn operator_mint_notification_success_preserves_locator_and_gross_separately() {
    let reply = bytes(
        "(variant { Ok = record { balance = 999 : nat; block_index = 42 : nat; minted = 110 : nat } })",
    );
    assert_eq!(
        decode_notification_reply(&reply).unwrap(),
        Notify::Minted {
            deposit_block_index: 42,
            gross_minted_cycles: 110,
            historical_balance_cycles: 999,
        }
    );
}

#[test]
fn operator_mint_notification_replies_preserve_unresolved_refund_and_history_states() {
    for (idl, expected) in [
        (
            "(variant { Err = variant { Processing } })",
            Notify::Processing,
        ),
        (
            "(variant { Err = variant { Refunded = record { block_index = opt (42 : nat64); reason = \"refund\" } } })",
            Notify::Refunded {
                refund_block_index: Some(42),
                reason: "refund".into(),
            },
        ),
        (
            "(variant { Err = variant { Refunded = record { block_index = null; reason = \"no block\" } } })",
            Notify::Refunded {
                refund_block_index: None,
                reason: "no block".into(),
            },
        ),
        (
            "(variant { Err = variant { TransactionTooOld = 42 : nat64 } })",
            Notify::TransactionTooOld {
                oldest_block_index: 42,
            },
        ),
        (
            "(variant { Err = variant { InvalidTransaction = \"wrong memo\" } })",
            Notify::InvalidTransaction {
                reason: "wrong memo".into(),
            },
        ),
        (
            "(variant { Err = variant { Other = record { error_code = 7 : nat64; error_message = \"unresolved\" } } })",
            Notify::Other {
                code: 7,
                message: "unresolved".into(),
            },
        ),
    ] {
        assert_eq!(decode_notification_reply(&bytes(idl)).unwrap(), expected);
    }
}

#[test]
fn operator_mint_unrepresentable_quantities_and_unknown_replies_fail_closed() {
    let max = u128::MAX.to_string();
    let overflow = "340282366920938463463374607431768211456";
    for (block, minted, balance) in [
        (overflow, "1", "1"),
        ("1", overflow, "1"),
        ("1", "1", overflow),
    ] {
        let idl = format!(
            "(variant {{ Ok = record {{ block_index = {block} : nat; minted = {minted} : nat; balance = {balance} : nat }} }})"
        );
        assert!(matches!(
            decode_notification_reply(&bytes(&idl)),
            Err(OperatorMintWireError::QuantityOverflow)
        ));
    }
    let idl = format!(
        "(variant {{ Ok = record {{ block_index = {max} : nat; minted = {max} : nat; balance = {max} : nat }} }})"
    );
    assert!(decode_notification_reply(&bytes(&idl)).is_ok());
    for reply in [
        vec![],
        bytes("(variant { Err = variant { UnknownFailure } })"),
    ] {
        assert!(matches!(
            decode_transfer_reply(&reply),
            Err(OperatorMintWireError::Candid(_))
        ));
        assert!(matches!(
            decode_notification_reply(&reply),
            Err(OperatorMintWireError::Candid(_))
        ));
    }
}
