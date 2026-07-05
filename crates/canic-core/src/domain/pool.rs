//! Module: domain::pool
//!
//! Responsibility: define pure pool value enums shared across storage
//! projections, workflow decisions, and endpoint DTOs.
//! Does not own: pool command/response DTO structs, stable pool records, or
//! pool scheduling workflows.
//! Boundary: DTOs re-export these values to preserve the public API path while
//! internal code imports them from the domain owner.

use candid::CandidType;
use serde::Deserialize;

///
/// CanisterPoolStatus
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum CanisterPoolStatus {
    PendingReset,
    Ready,
    Failed { reason: String },
}
