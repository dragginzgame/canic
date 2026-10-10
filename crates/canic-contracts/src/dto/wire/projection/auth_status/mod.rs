//! Passive auth status transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::auth::{ActiveDelegationProofStatusResponse, RootIssuerRenewalStatusResponse};
use candid::CandidType;
use serde::Deserialize;

/// Bounded RootStatusResponse wire projection for auth status.

#[derive(CandidType, Deserialize)]
pub enum RootStatusResponse {
    IssuerRenewal(RootIssuerRenewalStatusResponse),
}

/// Bounded CanisterStatusResponse wire projection for auth status.
#[derive(CandidType, Deserialize)]
pub enum CanisterStatusResponse {
    ActiveDelegationProof(ActiveDelegationProofStatusResponse),
}
