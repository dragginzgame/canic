//! Module: fleet_ensure::view::operator_mint
//!
//! Responsibility: expose a read-only funding estimate before a Fleet journal exists.
//! Boundary: this view neither approves ICP conversion nor becomes accounting evidence.

use serde::Serialize;

///
/// FreshOperatorFundingQuote
///
/// Host read-only estimate for a verified fresh plan, with explicit account and rate.
///

#[derive(Debug, Serialize)]
pub struct FreshOperatorFundingQuote {
    pub plan_sha256: String,
    pub operator: String,
    pub cycles_ledger: String,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub available_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub required_debit_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub shortfall_cycles: u128,
    pub estimated_icp_e8s: Option<u64>,
    pub rate: OperatorMintRateQuote,
}

///
/// OperatorMintRateQuote
///
/// Host read-only rate and fees; retained payment approval binds the final e8s.
///

#[derive(Clone, Debug, serde::Serialize)]
pub struct OperatorMintRateQuote {
    pub rate_timestamp_seconds: u64,
    pub xdr_permyriad_per_icp: u64,
    pub transfer_fee_e8s: u64,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub estimated_deposit_fee_cycles: u128,
}
