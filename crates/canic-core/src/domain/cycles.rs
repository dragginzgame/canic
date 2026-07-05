//! Module: domain::cycles
//!
//! Responsibility: define pure cycle-event value enums shared across storage
//! projections and endpoint DTOs.
//! Does not own: cycle event DTO structs, stable records, or runtime funding
//! workflows.
//! Boundary: DTOs re-export these values to preserve the public API path while
//! internal code imports them from the domain owner.

use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// CycleTopupEventStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[remain::sorted]
pub enum CycleTopupEventStatus {
    RequestErr,
    RequestOk,
    RequestScheduled,
}
