//! Passive pool observation transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use candid::CandidType;
use serde::Deserialize;

/// Bounded ManagedCanisterStatusRequest wire projection for pool observation.
#[derive(CandidType)]
pub enum ManagedCanisterStatusRequest {
    CycleBalance,
}

/// Bounded ManagedCanisterStatusResponse wire projection for pool observation.
#[derive(CandidType, Deserialize)]
pub enum ManagedCanisterStatusResponse {
    CycleBalance(crate::dto::role::CycleBalanceStatusResponse),
}
