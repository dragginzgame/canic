//! Passive delegation proof transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::auth::RootDelegationProofBatchProof;
use ic_platform::candid::contract::CandidType;

/// Bounded RootCommand wire projection for delegation proof.

#[derive(CandidType, serde::Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootCommand {
    GetOrCreateDelegationProof,
}

/// Bounded RootCommandResponse wire projection for delegation proof.
#[derive(CandidType, serde::Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootCommandResponse {
    GetOrCreateDelegationProof(RootDelegationProofBatchProof),
}
