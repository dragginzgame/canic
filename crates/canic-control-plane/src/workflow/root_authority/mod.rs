//! Module: workflow::root_authority
//!
//! Responsibility: load and validate this Canister's protected Fleet Subnet Root authority.
//! Does not own: activation mutation, Registry validation, or endpoint authorization.
//! Boundary: root workflows use this owner before consuming protected root state.

use candid::Principal;
use canic_core::{
    control_plane_support::{
        error::InternalError, ops::ic::IcOps,
        workflow::runtime::fleet_activation::FleetActivationWorkflow,
    },
    dto::fleet_subnet_root::FleetSubnetRootAuthority,
};

pub(super) fn validated_root_authority()
-> Result<(FleetSubnetRootAuthority, Principal), InternalError> {
    let authority = FleetActivationWorkflow::root_authority()?;
    let root = IcOps::canister_self();
    if authority.binding.fleet_subnet_root != root {
        return Err(InternalError::invalid_input(
            "protected Fleet Subnet Root authority does not name this Canister",
        ));
    }
    Ok((authority, root))
}
