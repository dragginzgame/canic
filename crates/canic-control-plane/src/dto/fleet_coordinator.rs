//! Module: dto::fleet_coordinator
//!
//! Responsibility: carry the protected fresh-install input for one Fleet Coordinator.
//! Does not own: validation, stable state, Registry compilation, or lifecycle effects.
//! Boundary: the Coordinator lifecycle adapter passes this passive payload to workflow.

use candid::CandidType;
use canic_core::{
    control_plane_support::config::ComponentDeploymentConfiguration,
    ids::{AppId, FleetRegistryAuthority},
};
use serde::Deserialize;

///
/// FleetCoordinatorInitArgs
///
/// Exact authority and compiled provisioning configuration installed into a fresh Coordinator.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetCoordinatorInitArgs {
    pub configured_app: AppId,
    pub authority: FleetRegistryAuthority,
    pub component_deployment_configuration: ComponentDeploymentConfiguration,
}
