//! Passive root admission transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::fleet_admission::{
    FleetAdmissionActivateTargetRequest, FleetAdmissionOpenTargetRequest,
    FleetAdmissionPrepareTargetRequest, FleetAdmissionTargetReceipt,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded RemoteManagedCommand wire projection for root admission.

#[derive(CandidType)]
pub enum RemoteManagedCommand {
    ActivateFleetAdmission(FleetAdmissionActivateTargetRequest),
    OpenFleetAdmission(FleetAdmissionOpenTargetRequest),
    PrepareFleetAdmission(Box<FleetAdmissionPrepareTargetRequest>),
}

/// Bounded RemoteManagedCommandResponse wire projection for root admission.
#[derive(CandidType, Deserialize)]
pub enum RemoteManagedCommandResponse {
    ActivateFleetAdmission(FleetAdmissionTargetReceipt),
    OpenFleetAdmission(FleetAdmissionTargetReceipt),
    PrepareFleetAdmission(FleetAdmissionTargetReceipt),
}
