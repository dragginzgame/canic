//! Passive coordinator client transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::{
    fleet_coordinator::CoordinatorRootRemovalOperationStatus,
    fleet_funding::{FleetRootFundingRequest, FleetRootFundingResponse},
    fleet_registry::{
        FleetRegistry, FleetSubnetRootSnapshotAcknowledgement,
        FleetSubnetRootSnapshotAcknowledgementRequest,
    },
    role::OperationStatusRequest,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded CoordinatorCommandFragment wire projection for coordinator client.

#[derive(CandidType)]
pub enum CoordinatorCommandFragment {
    AcknowledgeRootSnapshot(FleetSubnetRootSnapshotAcknowledgementRequest),
    RequestRootFunding(FleetRootFundingRequest),
}

/// Bounded CoordinatorCommandResponseFragment wire projection for coordinator client.
#[derive(CandidType, Deserialize)]
pub enum CoordinatorCommandResponseFragment {
    AcknowledgeRootSnapshot(FleetSubnetRootSnapshotAcknowledgement),
    RequestRootFunding(FleetRootFundingResponse),
}

/// Bounded CoordinatorStatusRequestFragment wire projection for coordinator client.
#[derive(CandidType)]
pub enum CoordinatorStatusRequestFragment {
    Operation(OperationStatusRequest),
    Registry,
}

/// Bounded CoordinatorStatusResponseFragment wire projection for coordinator client.
#[derive(CandidType, Deserialize)]
pub enum CoordinatorStatusResponseFragment {
    Operation(Box<CoordinatorOperationStatusFragment>),
    Registry(Box<FleetRegistry>),
}

/// Bounded CoordinatorOperationStatusFragment wire projection for coordinator client.
#[derive(CandidType, Deserialize)]
pub enum CoordinatorOperationStatusFragment {
    RootRemoval(CoordinatorRootRemovalOperationStatus),
}
