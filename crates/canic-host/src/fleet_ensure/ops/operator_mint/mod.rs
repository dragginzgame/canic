//! Module: fleet_ensure::ops::operator_mint
//!
//! Responsibility: construct exact mint intents and translate upstream wire data.
//! Boundary: receipt authentication stays in receipts; no approval or payment calls.

pub(in crate::fleet_ensure) mod journal;
pub mod receipts;
#[cfg(test)]
mod tests;
pub mod transport;
mod wire;

use crate::fleet_ensure::model::operator_mint::{
    OperatorMintAuthority, OperatorMintIntentRecord, OperatorMintNotificationOutcomeRecord,
    OperatorMintTransferOutcomeRecord,
};
use candid::{Nat, Principal};
use sha2_host::{Digest, Sha224, Sha256};
use thiserror::Error;

const MINT_MEMO: u64 = 0x544e_494d;
const DEPOSIT_MEMO_DOMAIN: &[u8] = b"canic:operator-mint:deposit:v1\0";

///
/// OperatorMintWireError
///
/// Ops-owned rejection of an inconsistent intent or unrepresentable reply.
/// Decode errors never establish that a submitted payment did not execute.
///

#[derive(Debug, Error)]
pub enum OperatorMintWireError {
    #[error("operator mint Candid encoding or decoding failed")]
    Candid(#[from] candid::Error),

    #[error("operator mint amount or timestamp is zero, or its debit overflows")]
    InvalidAmounts,

    #[error("operator mint authority includes an anonymous or management Principal")]
    InvalidAuthority,

    #[error("operator mint deposit memo differs from its retained intent")]
    MemoMismatch,

    #[error("operator mint reply quantity exceeds the bounded accounting range")]
    QuantityOverflow,
}

/// Construct a fixed default-account intent from already-reviewed inputs.
///
/// The journal owner must ensure timestamp uniqueness across its transfers and
/// persist approval and this exact intent before using either encoded request.
pub fn prepare_intent(
    authority: OperatorMintAuthority,
    amount_e8s: u64,
    transfer_fee_e8s: u64,
    created_at_time_ns: u64,
) -> Result<OperatorMintIntentRecord, OperatorMintWireError> {
    let mut intent = OperatorMintIntentRecord {
        authority,
        amount_e8s,
        transfer_fee_e8s,
        created_at_time_ns,
        deposit_memo: [0; 32],
    };
    validate_inputs(&intent)?;
    intent.deposit_memo = deposit_memo(&intent);
    Ok(intent)
}

/// Encode `transfer` using the retained amount, fee, timestamp and CMC account.
/// No retry path supplies a new clock sample or silently adjusts the fee.
pub fn transfer_argument(
    intent: &OperatorMintIntentRecord,
) -> Result<Vec<u8>, OperatorMintWireError> {
    validate_intent(intent)?;
    let subaccount = principal_subaccount(intent.authority.operator);
    let args = wire::TransferArgs {
        memo: MINT_MEMO,
        amount: wire::Tokens {
            e8s: intent.amount_e8s,
        },
        fee: wire::Tokens {
            e8s: intent.transfer_fee_e8s,
        },
        from_subaccount: None,
        to: account_identifier(intent.authority.cmc, &subaccount),
        created_at_time: Some(wire::Timestamp {
            timestamp_nanos: intent.created_at_time_ns,
        }),
    };
    Ok(candid::encode_one(args)?)
}

/// Encode `notify_mint_cycles` for the exact authenticated ICP transfer block.
/// The caller must bind that block to this intent before requesting notification.
pub fn notification_argument(
    intent: &OperatorMintIntentRecord,
    icp_block_index: u64,
) -> Result<Vec<u8>, OperatorMintWireError> {
    validate_intent(intent)?;
    Ok(candid::encode_one(wire::NotifyMintArgs {
        block_index: icp_block_index,
        deposit_memo: Some(intent.deposit_memo.to_vec()),
        to_subaccount: None,
    })?)
}

/// Decode an ICP reply without treating a duplicate or expiry as a new transfer.
pub fn decode_transfer_reply(
    bytes: &[u8],
) -> Result<OperatorMintTransferOutcomeRecord, OperatorMintWireError> {
    let result: Result<u64, wire::TransferError> = candid::decode_one(bytes)?;
    Ok(match result {
        Ok(block_index) => OperatorMintTransferOutcomeRecord::Accepted { block_index },
        Err(wire::TransferError::TxDuplicate { duplicate_of }) => {
            OperatorMintTransferOutcomeRecord::Duplicate {
                block_index: duplicate_of,
            }
        }
        Err(wire::TransferError::BadFee { expected_fee }) => {
            OperatorMintTransferOutcomeRecord::BadFee {
                expected_fee_e8s: expected_fee.e8s,
            }
        }
        Err(wire::TransferError::InsufficientFunds { balance }) => {
            OperatorMintTransferOutcomeRecord::InsufficientFunds {
                balance_e8s: balance.e8s,
            }
        }
        Err(wire::TransferError::TxCreatedInFuture) => {
            OperatorMintTransferOutcomeRecord::CreatedInFuture
        }
        Err(wire::TransferError::TxTooOld {
            allowed_window_nanos,
        }) => OperatorMintTransferOutcomeRecord::TooOld {
            allowed_window_ns: allowed_window_nanos,
        },
    })
}

/// Decode CMC evidence without trusting a cached balance or admitting any credit.
pub fn decode_notification_reply(
    bytes: &[u8],
) -> Result<OperatorMintNotificationOutcomeRecord, OperatorMintWireError> {
    let result: Result<wire::NotifyMintOk, wire::NotifyMintError> = candid::decode_one(bytes)?;
    Ok(match result {
        Ok(minted) => OperatorMintNotificationOutcomeRecord::Minted {
            deposit_block_index: bounded_nat(minted.block_index)?,
            gross_minted_cycles: bounded_nat(minted.minted)?,
            historical_balance_cycles: bounded_nat(minted.balance)?,
        },
        Err(wire::NotifyMintError::Processing) => OperatorMintNotificationOutcomeRecord::Processing,
        Err(wire::NotifyMintError::Refunded {
            block_index,
            reason,
        }) => OperatorMintNotificationOutcomeRecord::Refunded {
            refund_block_index: block_index,
            reason,
        },
        Err(wire::NotifyMintError::TransactionTooOld(oldest_block_index)) => {
            OperatorMintNotificationOutcomeRecord::TransactionTooOld { oldest_block_index }
        }
        Err(wire::NotifyMintError::InvalidTransaction(reason)) => {
            OperatorMintNotificationOutcomeRecord::InvalidTransaction { reason }
        }
        Err(wire::NotifyMintError::Other {
            error_code,
            error_message,
        }) => OperatorMintNotificationOutcomeRecord::Other {
            code: error_code,
            message: error_message,
        },
    })
}

fn validate_intent(intent: &OperatorMintIntentRecord) -> Result<(), OperatorMintWireError> {
    validate_inputs(intent)?;
    if intent.deposit_memo != deposit_memo(intent) {
        return Err(OperatorMintWireError::MemoMismatch);
    }
    Ok(())
}

fn validate_inputs(intent: &OperatorMintIntentRecord) -> Result<(), OperatorMintWireError> {
    if intent.amount_e8s == 0
        || intent.created_at_time_ns == 0
        || intent
            .amount_e8s
            .checked_add(intent.transfer_fee_e8s)
            .is_none()
    {
        return Err(OperatorMintWireError::InvalidAmounts);
    }
    for principal in [
        intent.authority.operator,
        intent.authority.icp_ledger,
        intent.authority.cmc,
        intent.authority.cycles_ledger,
    ] {
        if principal == Principal::anonymous() || principal == Principal::management_canister() {
            return Err(OperatorMintWireError::InvalidAuthority);
        }
    }
    Ok(())
}

fn deposit_memo(intent: &OperatorMintIntentRecord) -> [u8; 32] {
    let authority = &intent.authority;
    let mut hash = Sha256::new();
    hash.update(DEPOSIT_MEMO_DOMAIN);
    for binding in [
        authority.operation_id,
        authority.plan_sha256,
        authority.funding_review_sha256,
        authority.network_identity_sha256,
    ] {
        hash.update(binding);
    }
    // Canonical Principal text cannot contain NUL, making each field unambiguous.
    for principal in [
        authority.operator,
        authority.icp_ledger,
        authority.cmc,
        authority.cycles_ledger,
    ] {
        hash.update(principal.to_text().as_bytes());
        hash.update([0]);
    }
    for amount in [
        intent.amount_e8s,
        intent.transfer_fee_e8s,
        intent.created_at_time_ns,
    ] {
        hash.update(amount.to_be_bytes());
    }
    hash.finalize().into()
}

fn principal_subaccount(principal: Principal) -> [u8; 32] {
    let bytes = principal.as_slice();
    let mut subaccount = [0; 32];
    // The Principal type limits its representation to 29 bytes.
    subaccount[0] = u8::try_from(bytes.len()).expect("Principal length fits one byte");
    subaccount[1..][..bytes.len()].copy_from_slice(bytes);
    subaccount
}

fn account_identifier(owner: Principal, subaccount: &[u8; 32]) -> [u8; 32] {
    let mut hash = Sha224::new();
    hash.update(b"\x0aaccount-id");
    hash.update(owner.as_slice());
    hash.update(subaccount);
    let digest = hash.finalize();
    let mut account = [0; 32];
    account[..4].copy_from_slice(&crc32fast::hash(&digest).to_be_bytes());
    account[4..].copy_from_slice(&digest);
    account
}

fn bounded_nat(value: Nat) -> Result<u128, OperatorMintWireError> {
    value
        .0
        .try_into()
        .map_err(|_| OperatorMintWireError::QuantityOverflow)
}
