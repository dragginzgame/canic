//! Module: domain::metrics
//!
//! Responsibility: define pure metric-family selector values shared by runtime
//! metric projection, workflow queries, and endpoint DTOs.
//! Does not own: metric row DTO structs, metric recording, or CLI metrics
//! transport models.
//! Boundary: DTOs re-export these values to preserve the public API path while
//! internal code imports them from the domain owner.

use candid::CandidType;
use serde::Deserialize;

///
/// MetricsKind
///
/// Metric tier selector.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize)]
#[remain::sorted]
pub enum MetricsKind {
    Core,
    Placement,
    Platform,
    Runtime,
    Security,
    Storage,
}
