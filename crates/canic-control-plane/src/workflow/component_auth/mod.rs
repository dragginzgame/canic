//! Module: workflow::component_auth
//!
//! Responsibility: resolve active root-owned Component Registry membership.
//! Does not own: endpoint predicates, Component Registry persistence, or proof creation.
//! Boundary: combines active root runtime authority with exact Registry membership.

use canic_core::{
    control_plane_support::{
        error::InternalError, workflow::runtime::fleet_activation::FleetActivationWorkflow,
    },
    dto::error::Error,
    ids::ManagedCanisterBinding,
};

pub use super::component_registry::ActiveComponentMemberError;

/// Resolve one exact active member under an active Fleet Subnet Root.
pub fn active_component_member(caller: candid::Principal) -> Result<ManagedCanisterBinding, Error> {
    active_component_member_for_access(caller)
        .map_err(InternalError::from)
        .map_err(Into::into)
}

/// Preserve runtime and Registry causes while evaluating endpoint access.
pub fn active_component_member_for_access(
    caller: candid::Principal,
) -> Result<ManagedCanisterBinding, ActiveComponentMemberError> {
    require_active_fleet_subnet_root_internal()?;
    super::component_registry::active_component_member(caller)
}

/// Resolve an exact Prepared or Active Registry member after protected Root validation.
pub fn registered_component_member_for_access(
    caller: candid::Principal,
) -> Result<ManagedCanisterBinding, ActiveComponentMemberError> {
    Ok(super::component_registry::registered_component_member_authority(caller)?.binding)
}

/// Require the local Fleet Subnet Root runtime to be active.
pub fn require_active_fleet_subnet_root() -> Result<(), Error> {
    require_active_fleet_subnet_root_internal().map_err(Into::into)
}

/// Preserve the exact activation failure for endpoint access predicates.
pub fn require_active_fleet_subnet_root_internal() -> Result<(), InternalError> {
    FleetActivationWorkflow::require_active()
}

/// Require the exact pre-activation Root phase used only by compiled initial-child bootstrap.
pub fn require_prepared_fleet_subnet_root() -> Result<(), Error> {
    let status = FleetActivationWorkflow::status().map_err(Error::from)?;
    if status.phase != canic_core::dto::fleet_activation::FleetActivationPhase::Prepared {
        return Err(Error::from_registered(
            canic_core::diagnostics::codes::LIFECYCLE_INACTIVE,
        ));
    }
    Ok(())
}
