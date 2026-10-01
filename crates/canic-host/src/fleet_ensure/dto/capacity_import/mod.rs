//! Explicit operator input for a current-Fleet capacity review.

use candid::Principal;
use serde::Serialize;
use std::path::PathBuf;

/// Review inputs; no source, debit allowance or destination is inferred from a journal edit.
#[derive(Clone, Debug, Serialize)]
pub struct CapacityImportReviewRequest {
    /// Additional credits to observe before approval; the empty request retains its survey identity.
    pub funding_credits: Vec<CapacityImportFundingCreditRequest>,
    pub environment: String,
    pub fleet: String,
    pub canisters: Vec<Principal>,
    pub root: Option<Principal>,
    pub declarations: PathBuf,
    pub policy: PathBuf,
    pub seed: PathBuf,
    pub maximum_source_debit_cycles: u128,
    pub maximum_root_debit_cycles: u128,
    pub maximum_root_paid_calls: u32,
}

/// Operator-declared additional funding; review must bind both the original and current samples.
#[derive(Clone, Debug, Serialize)]
pub struct CapacityImportFundingCreditRequest {
    pub canister: Principal,
    pub cycles: u128,
}
