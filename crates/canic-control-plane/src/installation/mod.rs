//! Exact Coordinator installation contract coupled to compiled runtime configuration.
//!
//! Runtime configuration compilation remains owned by Core.

use candid::CandidType;
use canic_contracts::ids::{
    AppId, FleetAdmissionPolicy, FleetCoordinatorRootFundingPolicy, FleetRegistryAuthority,
};
use canic_core::control_plane_support::config::ComponentDeploymentConfiguration;
use serde::Deserialize;

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetCoordinatorInitArgs {
    pub configured_app: AppId,
    pub authority: FleetRegistryAuthority,
    pub admission: FleetAdmissionPolicy,
    pub component_deployment_configuration: ComponentDeploymentConfiguration,
    pub root_funding: Option<FleetCoordinatorRootFundingPolicy>,
}
