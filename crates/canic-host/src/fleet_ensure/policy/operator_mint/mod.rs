//! Module: fleet_ensure::policy::operator_mint
//!
//! Responsibility: check exact conversion receipt bindings and bounded accounting.
//! Boundary: pure checks do not authenticate Ledger evidence or approve any effect.

pub(in crate::fleet_ensure) mod progress;
pub mod quote;
#[cfg(test)]
mod tests;

use crate::fleet_ensure::model::FleetEnsureJournalRecord;
use crate::fleet_ensure::model::operator_mint::{
    OperatorMintIntentRecord, OperatorMintReceiptRecord,
};
use thiserror::Error;

///
/// OperatorMintReceiptError
///
/// Policy-owned structural receipt rejection, separate from authentication.
///

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum OperatorMintReceiptError {
    #[error("operator mint gross amount does not equal positive net credit plus deposit fee")]
    CycleAmountMismatch,

    #[error("operator mint credit and deposit fee exceed the accounting range")]
    CycleAmountOverflow,

    #[error("operator mint deposit differs from the retained destination or memo")]
    DepositMismatch,

    #[error("operator mint deposit receipt was already admitted")]
    DuplicateDeposit,

    #[error("operator mint ICP transfer receipt was already admitted")]
    DuplicateTransfer,

    #[error("operator mint ICP debit exceeds the accounting range")]
    IcpDebitOverflow,

    #[error("operator mint receipt differs from the retained intent")]
    IntentMismatch,

    #[error("operator mint intent has zero amount or creation time")]
    InvalidIntent,
}

///
/// OperatorMintCredit
///
/// Policy-checked amounts for the separate ICP and operator-cycle equations.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperatorMintCredit {
    pub icp_debit_e8s: u64,
    pub net_credit_cycles: u128,
}

/// Validate authenticated receipt facts without mutating starting balances or history.
///
/// This only checks structural binding and arithmetic. The future transport owner
/// must verify both Ledger transactions before supplying these records, and the
/// journal owner must atomically retain an admitted receipt with its credit.
pub fn admit_receipt(
    intent: &OperatorMintIntentRecord,
    receipt: &OperatorMintReceiptRecord,
    admitted: &[OperatorMintReceiptRecord],
) -> Result<OperatorMintCredit, OperatorMintReceiptError> {
    if intent.amount_e8s == 0 || intent.created_at_time_ns == 0 {
        return Err(OperatorMintReceiptError::InvalidIntent);
    }
    let icp_debit_e8s = intent
        .amount_e8s
        .checked_add(intent.transfer_fee_e8s)
        .ok_or(OperatorMintReceiptError::IcpDebitOverflow)?;
    if &receipt.intent != intent {
        return Err(OperatorMintReceiptError::IntentMismatch);
    }
    if receipt.destination_owner != intent.authority.operator
        || receipt.destination_subaccount.is_some()
        || receipt.deposit_memo != intent.deposit_memo
    {
        return Err(OperatorMintReceiptError::DepositMismatch);
    }
    for previous in admitted {
        if previous.intent.authority.network_identity_sha256
            != intent.authority.network_identity_sha256
        {
            continue;
        }
        if previous.intent.authority.icp_ledger == intent.authority.icp_ledger
            && previous.icp_block_index == receipt.icp_block_index
        {
            return Err(OperatorMintReceiptError::DuplicateTransfer);
        }
        if previous.intent.authority.cycles_ledger == intent.authority.cycles_ledger
            && previous.deposit_block_index == receipt.deposit_block_index
        {
            return Err(OperatorMintReceiptError::DuplicateDeposit);
        }
    }
    let gross = receipt
        .net_credit_cycles
        .checked_add(receipt.deposit_fee_cycles)
        .ok_or(OperatorMintReceiptError::CycleAmountOverflow)?;
    if receipt.net_credit_cycles == 0 || gross != receipt.gross_minted_cycles {
        return Err(OperatorMintReceiptError::CycleAmountMismatch);
    }
    Ok(OperatorMintCredit {
        icp_debit_e8s,
        net_credit_cycles: receipt.net_credit_cycles,
    })
}

/// Account for authenticated, once-only mint credits without changing the baseline.
/// Journal verification must validate every retained receipt before using this sum.
pub(in crate::fleet_ensure) fn operator_source(journal: &FleetEnsureJournalRecord) -> Option<u128> {
    journal
        .funding_reviews
        .iter()
        .filter_map(|review| review.operator_mint.as_ref()?.receipt.as_ref())
        .try_fold(journal.initial_operator_cycles, |source, receipt| {
            source.checked_add(receipt.net_credit_cycles)
        })
}
