//! Module: api::component_auth
//!
//! Responsibility: authenticate active Components at root-owned auth endpoints.
//! Does not own: Component Registry state, proof state, or attestation policy.
//! Boundary: resolves protected caller authority, then delegates to core auth workflow.

use canic_core::{
    api::{auth::AuthApi, fleet_activation::FleetActivationApi},
    control_plane_support::ops::ic::IcOps,
    dto::{
        auth::{
            RoleAttestationGetRequest, RoleAttestationPrepareResponse, RoleAttestationRequest,
            SignedRoleAttestation,
        },
        error::Error,
        fleet_activation::FleetActivationPhase,
    },
    ids::ComponentBinding,
};

///
/// ComponentAuthApi
///
/// Root-owned auth services admitted only for active Component callers.
///

pub struct ComponentAuthApi;

impl ComponentAuthApi {
    /// Prepare a role attestation for the exact calling active Component.
    pub fn prepare_role_attestation(
        request: RoleAttestationRequest,
    ) -> Result<RoleAttestationPrepareResponse, Error> {
        let component = active_component_caller()?;
        AuthApi::prepare_component_role_attestation_root(request, &component)
    }

    /// Retrieve caller-bound proof only while the Component remains active.
    pub fn get_role_attestation(
        request: RoleAttestationGetRequest,
    ) -> Result<SignedRoleAttestation, Error> {
        active_component_caller()?;
        AuthApi::get_role_attestation_root(request)
    }
}

fn active_component_caller() -> Result<ComponentBinding, Error> {
    let activation = FleetActivationApi::status()?;
    if activation.phase != FleetActivationPhase::Active {
        return Err(Error::unavailable(
            "role attestation requires an Active Fleet Subnet Root",
        ));
    }
    crate::workflow::component_registry::active_component_binding(IcOps::msg_caller())
        .map_err(Into::into)
}
