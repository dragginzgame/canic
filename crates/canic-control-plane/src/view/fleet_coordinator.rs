//! Module: view::fleet_coordinator
//!
//! Responsibility: expose read-only Coordinator provisioning decisions to workflow.
//! Does not own: stable records, inter-canister calls, or Registry publication.
//! Boundary: ops returns one exact current result or root acceptance call authority.

use candid::Principal;
use canic_core::dto::component_provisioning::{
    FleetComponentProvisioningStatusResponse, RootComponentProvisioningAcceptanceRequest,
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
