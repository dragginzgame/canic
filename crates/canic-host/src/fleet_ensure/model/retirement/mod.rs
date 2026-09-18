//! Module: fleet_ensure::model::retirement
//!
//! Responsibility: retain immutable retirement accounting in its recorded vocabulary.
//! Does not own: live conservation, funding admission or executable predecessor plans.
//! Boundary: evidence is hashed without conversion; fresh observations authorize effects.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::model::{ActualCycleConservation, u128_text};
use serde::{Deserialize, Serialize};

/// Accounting evidence embedded in an already reviewed retirement.
///
/// Both shapes preserve their original canonical field order and numeric encoding.
/// The recorded execution observation is never converted into a live net balance
/// report: its labels do not establish the semantics of today's observations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum FleetRetirementConservationRecord {
    NetBalance(ActualCycleConservation),
    RecordedExecution(RecordedExecutionConservationRecord),
}

/// Frozen accounting evidence from a completed retirement; never payment authority.
/// Field order is part of the retained plan's original digest contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedExecutionConservationRecord {
    #[serde(with = "u128_text")]
    estate_funding_cycles: u128,
    #[serde(with = "u128_text")]
    exact_estate_creation_fee_cycles: u128,
    #[serde(with = "u128_text")]
    exact_unavoidable_fee_cycles: u128,
    #[serde(with = "u128_text")]
    final_controlled_cycles: u128,
    #[serde(with = "u128_text")]
    measured_execution_burn_cycles: u128,
    #[serde(with = "u128_text")]
    observed_starting_cycles: u128,
    #[serde(with = "u128_text")]
    observed_settlement_credit_cycles: u128,
    #[serde(with = "u128_text")]
    operator_debit_cycles: u128,
    #[serde(with = "u128_text")]
    received_new_funding_cycles: u128,
}
