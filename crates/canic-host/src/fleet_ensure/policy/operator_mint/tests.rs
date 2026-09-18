//! Receipt binding, duplicate accounting and arithmetic boundary tests.

use super::{OperatorMintCredit, OperatorMintReceiptError, admit_receipt};
use crate::fleet_ensure::model::operator_mint::{
    OperatorMintAuthority, OperatorMintIntentRecord, OperatorMintReceiptRecord,
};
use candid::Principal;

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
            amount_e8s: 100_000_000,
            transfer_fee_e8s: 10_000,
            created_at_time_ns: 42,
            deposit_memo: [5; 32],
        },
        icp_block_index: 7,
        deposit_block_index: 8,
        destination_owner: Principal::from_slice(&[1, 1]),
        destination_subaccount: None,
        deposit_memo: [5; 32],
        gross_minted_cycles: 1_100,
        deposit_fee_cycles: 100,
        net_credit_cycles: 1_000,
    }
}

#[test]
fn operator_mint_admits_only_net_credit_and_exact_icp_debit() {
    let receipt = receipt();
    assert_eq!(
        admit_receipt(&receipt.intent, &receipt, &[]),
        Ok(OperatorMintCredit {
            icp_debit_e8s: 100_010_000,
            net_credit_cycles: 1_000,
        })
    );
}

#[test]
fn operator_mint_rejects_every_changed_intent_binding() {
    let receipt = receipt();
    let mutations: [fn(&mut OperatorMintIntentRecord); 12] = [
        |r| r.authority.operation_id[0] ^= 1,
        |r| r.authority.plan_sha256[0] ^= 1,
        |r| r.authority.funding_review_sha256[0] ^= 1,
        |r| r.authority.network_identity_sha256[0] ^= 1,
        |r| r.authority.operator = Principal::anonymous(),
        |r| r.authority.icp_ledger = Principal::anonymous(),
        |r| r.authority.cmc = Principal::anonymous(),
        |r| r.authority.cycles_ledger = Principal::anonymous(),
        |r| r.amount_e8s += 1,
        |r| r.transfer_fee_e8s += 1,
        |r| r.created_at_time_ns += 1,
        |r| r.deposit_memo[0] ^= 1,
    ];
    for mutate in mutations {
        let mut changed = receipt.clone();
        mutate(&mut changed.intent);
        assert_eq!(
            admit_receipt(&receipt.intent, &changed, &[]),
            Err(OperatorMintReceiptError::IntentMismatch)
        );
    }
}

#[test]
fn operator_mint_rejects_unbound_deposit() {
    let receipt = receipt();
    let mutations: [fn(&mut OperatorMintReceiptRecord); 3] = [
        |r| r.destination_owner = Principal::anonymous(),
        |r| r.destination_subaccount = Some([6; 32]),
        |r| r.deposit_memo[0] ^= 1,
    ];
    for mutate in mutations {
        let mut changed = receipt.clone();
        mutate(&mut changed);
        assert_eq!(
            admit_receipt(&receipt.intent, &changed, &[]),
            Err(OperatorMintReceiptError::DepositMismatch)
        );
    }
}

#[test]
fn operator_mint_rejects_reused_blocks_across_operations() {
    let receipt = receipt();
    let mut previous = receipt.clone();
    previous.intent.authority.operation_id = [9; 32];
    previous.intent.deposit_memo = [9; 32];
    previous.deposit_block_index += 1;
    assert_eq!(
        admit_receipt(&receipt.intent, &receipt, &[previous]),
        Err(OperatorMintReceiptError::DuplicateTransfer)
    );
    let mut previous = receipt.clone();
    previous.intent.authority.operation_id = [9; 32];
    previous.icp_block_index += 1;
    assert_eq!(
        admit_receipt(&receipt.intent, &receipt, &[previous]),
        Err(OperatorMintReceiptError::DuplicateDeposit)
    );
}

#[test]
fn operator_mint_block_identity_includes_network_and_ledger() {
    let receipt = receipt();
    let mut other_network = receipt.clone();
    other_network.intent.authority.network_identity_sha256 = [9; 32];
    let mut other_ledgers = receipt.clone();
    other_ledgers.intent.authority.icp_ledger = Principal::from_slice(&[6]);
    other_ledgers.intent.authority.cycles_ledger = Principal::from_slice(&[7]);
    assert!(admit_receipt(&receipt.intent, &receipt, &[other_network, other_ledgers]).is_ok());
}

#[test]
fn operator_mint_rejects_invalid_intent_and_icp_overflow() {
    let mut receipt = receipt();
    receipt.intent.amount_e8s = 0;
    assert_eq!(
        admit_receipt(&receipt.intent, &receipt, &[]),
        Err(OperatorMintReceiptError::InvalidIntent)
    );
    receipt.intent.amount_e8s = 1;
    receipt.intent.created_at_time_ns = 0;
    assert_eq!(
        admit_receipt(&receipt.intent, &receipt, &[]),
        Err(OperatorMintReceiptError::InvalidIntent)
    );
    receipt.intent.created_at_time_ns = 1;
    receipt.intent.amount_e8s = u64::MAX;
    assert_eq!(
        admit_receipt(&receipt.intent, &receipt, &[]),
        Err(OperatorMintReceiptError::IcpDebitOverflow)
    );
}

#[test]
fn operator_mint_checks_cycle_equation_without_saturation() {
    let mut receipt = receipt();
    receipt.net_credit_cycles = u128::MAX;
    assert_eq!(
        admit_receipt(&receipt.intent, &receipt, &[]),
        Err(OperatorMintReceiptError::CycleAmountOverflow)
    );
    receipt.net_credit_cycles = 0;
    receipt.gross_minted_cycles = receipt.deposit_fee_cycles;
    assert_eq!(
        admit_receipt(&receipt.intent, &receipt, &[]),
        Err(OperatorMintReceiptError::CycleAmountMismatch)
    );
    receipt.net_credit_cycles = 1;
    assert_eq!(
        admit_receipt(&receipt.intent, &receipt, &[]),
        Err(OperatorMintReceiptError::CycleAmountMismatch)
    );
    receipt.net_credit_cycles = u128::MAX - receipt.deposit_fee_cycles;
    receipt.gross_minted_cycles = u128::MAX;
    receipt.intent.amount_e8s = u64::MAX - receipt.intent.transfer_fee_e8s;
    assert_eq!(
        admit_receipt(&receipt.intent, &receipt, &[]),
        Ok(OperatorMintCredit {
            icp_debit_e8s: u64::MAX,
            net_credit_cycles: u128::MAX - receipt.deposit_fee_cycles,
        })
    );
}
