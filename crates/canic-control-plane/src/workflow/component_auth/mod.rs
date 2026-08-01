//! Module: workflow::component_auth
//!
//! Responsibility: resolve active root-owned Component Registry membership.
//! Does not own: endpoint predicates, Component Registry persistence, or proof creation.
//! Boundary: combines active root runtime authority with exact Registry membership.

use canic_core::{
    control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow,
    dto::error::Error, ids::ManagedCanisterBinding,
};

/// Resolve one exact active member under an active Fleet Subnet Root.
pub fn active_component_member(caller: candid::Principal) -> Result<ManagedCanisterBinding, Error> {
    require_active_fleet_subnet_root()?;
    super::component_registry::active_component_member(caller).map_err(Into::into)
}

/// Require the local Fleet Subnet Root runtime to be active.
pub fn require_active_fleet_subnet_root() -> Result<(), Error> {
    FleetActivationWorkflow::require_active().map_err(Into::into)
}
