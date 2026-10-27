//! Module: domain::cycles
//!
//! Responsibility: define pure cycle-event value enums shared across storage
//! projections and endpoint DTOs.
//! Does not own: cycle event DTO structs, stable records, or runtime funding
//! workflows.
//! Boundary: DTOs re-export these values to preserve the public API path while
//! internal code imports them from the domain owner.

use serde::{Deserialize, Serialize};

///
/// CycleTopupEventStatus
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
#[remain::sorted]
pub enum CycleTopupEventStatus {
    RequestErr,
    RequestOk,
    RequestScheduled,
}

/// Scheduling decision after a failed automatic parent funding request.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum CycleTopupFailureDisposition {
    Retry,
    Terminal,
}
