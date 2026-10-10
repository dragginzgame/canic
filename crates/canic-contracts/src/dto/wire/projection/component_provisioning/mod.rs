//! Passive component provisioning transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use candid::CandidType;
use serde::Deserialize;

/// Bounded RemoteCoordinatorStatusResponse wire projection for component provisioning.
#[derive(CandidType, Deserialize)]
pub enum RemoteCoordinatorStatusResponse {
    Operation(RemoteCoordinatorOperationStatusResponse),
}

/// Bounded RemoteCoordinatorOperationStatusResponse wire projection for component provisioning.
#[derive(CandidType, Deserialize)]
pub enum RemoteCoordinatorOperationStatusResponse {
    ComponentProvisioning(
        crate::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
    ),
}
