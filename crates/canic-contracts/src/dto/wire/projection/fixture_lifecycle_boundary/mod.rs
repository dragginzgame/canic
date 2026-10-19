//! Bounded Canic transports for lifecycle boundary validation.
//!
//! Keep command tables and reply decoding limited to the exercised methods.
//! Canonical compatibility checks cover every retained selector and payload.

use crate::dto::{
    runtime::CanicReadinessStatus,
    wire::projection::component_registry::CanisterOperationStatusFragment as CanisterOperationStatusResponse,
};

/// Bounded CanisterStatusResponse selectors for lifecycle boundary validation.
#[derive(ic_platform::candid::contract::CandidType, serde::Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum CanisterStatusResponse {
    Operation(Box<CanisterOperationStatusResponse>),
    Readiness(CanicReadinessStatus),
}
