//! Module: fleet_ensure::view
//!
//! Responsibility: expose read-only funding projections for operator review.
//! Does not own: persisted authority, funding admission or cycle effects.
//! Boundary: projections describe assumptions and never authorize spending.

pub mod continuation;
pub mod operator_mint;
pub mod readiness;
pub mod startup_funding;
pub(in crate::fleet_ensure) mod terminal_source;

/// Live operator account and fee at the configured Cycles Ledger.
///
/// Host workflows use this observation to check reviewed funding before effects;
/// it carries no payment authority or runtime protocol evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorFundingObservation {
    pub cycles_ledger: String,
    pub ledger_fee_cycles: u128,
    pub operator_cycles: u128,
}
