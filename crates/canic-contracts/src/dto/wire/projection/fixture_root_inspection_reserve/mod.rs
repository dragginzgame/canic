//! Bounded Canic transports for root inspection reserve validation.
//!
//! Keep command tables and reply decoding limited to the exercised methods.
//! Canonical compatibility checks cover every retained selector and payload.

use crate::dto::canister::CanisterInspectionRequest;

/// Bounded ReserveRequest selectors for root inspection reserve validation.
#[derive(candid::CandidType)]
pub enum ReserveRequest {
    CycleBalance,
    InspectionReserve(CanisterInspectionRequest),
}
