//! Module: view::fleet_coordinator
//!
//! Responsibility: expose read-only Coordinator provisioning decisions to workflow.
//! Does not own: stable records, inter-canister calls, or Registry publication.
//! Boundary: ops returns one exact current result or root call authority.

use candid::Principal;
use canic_core::dto::component_provisioning::{
    FleetComponentProvisioningStatusResponse, RootComponentActivationRequest,
    RootComponentDirectorySynchronizationRequest, RootComponentProvisioningAcceptanceRequest,
    RootComponentProvisioningAdvanceRequest, RootComponentPublicationRequest,
};

/// One exact root acceptance call derived only from the durable Coordinator plan.
pub struct FleetComponentProvisioningRootAcceptanceCallView {
    pub fleet_subnet_root: Principal,
    pub request: RootComponentProvisioningAcceptanceRequest,
}

/// Coordinator decision for one expected root-acceptance cursor.
pub enum FleetComponentProvisioningRootAcceptanceDisposition {
    Current(FleetComponentProvisioningStatusResponse),
    Invoke(FleetComponentProvisioningRootAcceptanceCallView),
    Reconcile(FleetComponentProvisioningRootAcceptanceCallView),
}

/// One exact root advance call derived only from durable Coordinator progress.
pub struct FleetComponentProvisioningRootProvisionCallView {
    pub fleet_subnet_root: Principal,
    pub request: RootComponentProvisioningAdvanceRequest,
}

/// Coordinator decision for one expected root provisioning cursor.
pub enum FleetComponentProvisioningRootProvisionDisposition {
    Current(Box<FleetComponentProvisioningStatusResponse>),
    Invoke(FleetComponentProvisioningRootProvisionCallView),
    Publish,
    Reconcile(FleetComponentProvisioningRootProvisionCallView),
}

/// One exact root publication call derived only from durable Coordinator state.
pub enum FleetComponentDirectoryConfirmationCallView {
    FreshPublication {
        fleet_subnet_root: Principal,
        request: RootComponentPublicationRequest,
    },
    ScaleOutSynchronization {
        fleet_subnet_root: Principal,
        request: RootComponentDirectorySynchronizationRequest,
    },
    ScaleOutPublication {
        fleet_subnet_root: Principal,
        request: RootComponentPublicationRequest,
    },
}

/// Coordinator decision for one expected Directory-confirmation cursor.
pub enum FleetComponentDirectoryConfirmationDisposition {
    Current(Box<FleetComponentProvisioningStatusResponse>),
    Invoke(FleetComponentDirectoryConfirmationCallView),
    Reconcile(FleetComponentDirectoryConfirmationCallView),
}

/// One exact root runtime-activation call derived only from durable Coordinator state.
pub struct FleetComponentRuntimeActivationCallView {
    pub fleet_subnet_root: Principal,
    pub request: RootComponentActivationRequest,
}

/// Coordinator decision for one expected root runtime-activation cursor.
pub enum FleetComponentRuntimeActivationDisposition {
    Current(Box<FleetComponentProvisioningStatusResponse>),
    Invoke(FleetComponentRuntimeActivationCallView),
    Reconcile(FleetComponentRuntimeActivationCallView),
}
