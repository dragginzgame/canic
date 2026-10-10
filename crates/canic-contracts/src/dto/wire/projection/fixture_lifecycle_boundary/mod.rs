//! Bounded Canic transports for lifecycle boundary validation.
//!
//! Keep command tables and reply decoding limited to the exercised methods.
//! Canonical compatibility checks cover every retained selector and payload.

use crate::dto::{
    runtime::CanicReadinessStatus,
    wire::projection::component_registry::CanisterOperationStatusFragment as CanisterOperationStatusResponse,
};

/// Bounded CanisterStatusResponse selectors for lifecycle boundary validation.
#[derive(candid::CandidType, serde::Deserialize)]
pub enum CanisterStatusResponse {
    Operation(Box<CanisterOperationStatusResponse>),
    Readiness(CanicReadinessStatus),
}
