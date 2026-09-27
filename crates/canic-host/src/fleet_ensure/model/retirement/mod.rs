//! Module: fleet_ensure::model::retirement
//!
//! Responsibility: retain immutable current-contract retirement accounting.
//! Does not own: live conservation, funding admission or executable predecessor plans.
//! Boundary: evidence is hashed without conversion; fresh observations authorize effects.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::model::{ActualCycleConservation, u128_text};
use serde::{Deserialize, Serialize};

/// Accounting evidence embedded in an already reviewed retirement.
///
/// Exact native balances and reviewed external debits remain bound to the source receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum FleetRetirementConservationRecord {
    ExternalDebit(Box<RetirementExternalDebitRecord>),
    NetBalance(ActualCycleConservation),
}

/// Separate reviewed accounting for one external operator withdrawal; never Fleet funding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetirementExternalDebitRecord {
    pub source_conservation: ActualCycleConservation,
    pub external_debit: RetirementWithdrawalRecord,
}

/// Exact replicated Ledger block and controlled destination bound into retirement approval.
/// The burn proves the operator debit, not delivery or permission to retry the withdrawal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetirementWithdrawalRecord {
    pub ledger: String,
    pub operator: String,
    pub destination: String,
    pub network_identity_sha256: String,
    pub block_sha256: String,
    pub block_index: u64,
    pub timestamp_ns: u64,
    #[serde(with = "u128_text")]
    pub amount_cycles: u128,
    #[serde(with = "u128_text")]
    pub fee_cycles: u128,
}
