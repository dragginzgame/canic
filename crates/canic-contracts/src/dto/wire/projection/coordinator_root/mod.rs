//! Passive coordinator root transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::{
    component_provisioning::{
        RootComponentDirectorySynchronizationRequest,
        RootComponentDirectorySynchronizationResponse, RootComponentProvisioningAcceptanceRequest,
        RootComponentProvisioningStatusResponse,
    },
    fleet_admission::{
        FleetAdmissionActivateRootRequest, FleetAdmissionOpenRootRequest,
        FleetAdmissionPrepareRootRequest, FleetAdmissionRootReceipt,
    },
    fleet_funding::{
        FleetFundingPolicyRotationRootActivateRequest,
        FleetFundingPolicyRotationRootPrepareRequest, FleetFundingPolicyRotationRootReceipt,
        FleetRootFundingAcceptanceReceipt,
    },
    role::{OperationReceipt, RootRemovalRequest},
    root::RootRemovalOperationStatus,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded RemoteRootCommand wire projection for coordinator root.

#[derive(CandidType)]
pub enum RemoteRootCommand {
    AcceptFunding(crate::dto::fleet_funding::FleetRootFundingAcceptanceRequest),
    ActivateFleetAdmission(FleetAdmissionActivateRootRequest),
    ActivateFundingPolicyRotation(FleetFundingPolicyRotationRootActivateRequest),
    OpenFleetAdmission(FleetAdmissionOpenRootRequest),
    PrepareFleetAdmission(FleetAdmissionPrepareRootRequest),
    PrepareFundingPolicyRotation(FleetFundingPolicyRotationRootPrepareRequest),
    ProvisionComponents(RootComponentProvisioningAcceptanceRequest),
    RemoveRoot(RootRemovalRequest),
    SynchronizeComponentDirectories(RootComponentDirectorySynchronizationRequest),
}

/// Bounded RemoteRootCommandResponse wire projection for coordinator root.
#[derive(CandidType, Deserialize)]
pub enum RemoteRootCommandResponse {
    AcceptFunding(Box<FleetRootFundingAcceptanceReceipt>),
    ActivateFleetAdmission(Box<FleetAdmissionRootReceipt>),
    ActivateFundingPolicyRotation(Box<FleetFundingPolicyRotationRootReceipt>),
    OpenFleetAdmission(Box<FleetAdmissionRootReceipt>),
    OperationAccepted(Box<OperationReceipt>),
    PrepareFleetAdmission(Box<FleetAdmissionRootReceipt>),
    PrepareFundingPolicyRotation(Box<FleetFundingPolicyRotationRootReceipt>),
    SynchronizeComponentDirectories(Box<RootComponentDirectorySynchronizationResponse>),
}

/// Bounded RemoteRootStatusResponse wire projection for coordinator root.
#[derive(CandidType, Deserialize)]
pub enum RemoteRootStatusResponse {
    Operation(RemoteRootOperationStatusResponse),
}

/// Bounded RemoteRootOperationStatusResponse wire projection for coordinator root.
#[derive(CandidType, Deserialize)]
pub enum RemoteRootOperationStatusResponse {
    ProvisionComponents(Box<RootComponentProvisioningStatusResponse>),
    RemoveRoot(Box<RootRemovalOperationStatus>),
}
