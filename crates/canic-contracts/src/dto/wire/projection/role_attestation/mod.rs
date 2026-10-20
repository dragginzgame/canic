//! Passive role attestation transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::auth::{InstallActiveDelegationProofRequest, InstallActiveDelegationProofResponse};
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded IssuerCommandFragment wire projection for role attestation.

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum IssuerCommandFragment {
    InstallDelegationProof(InstallActiveDelegationProofRequest),
}

/// Bounded IssuerCommandResponseFragment wire projection for role attestation.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum IssuerCommandResponseFragment {
    InstallDelegationProof(InstallActiveDelegationProofResponse),
}
