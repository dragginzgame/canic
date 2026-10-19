//! Passive store activation transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::fleet_activation::{
    FleetActivationRequest, FleetActivationStatusResponse, FleetCredentialGenerationRequest,
};
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded StoreCommandFragment wire projection for store activation.

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum StoreCommandFragment {
    ActivateFleet(FleetActivationRequest),
    PrepareFleetCredential(FleetCredentialGenerationRequest),
}

/// Bounded StoreStatusResponseFragment wire projection for store activation.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum StoreStatusResponseFragment {
    Operation(StoreOperationStatusFragment),
}

/// Bounded StoreOperationStatusFragment wire projection for store activation.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum StoreOperationStatusFragment {
    FleetActivation(FleetActivationStatusResponse),
}
