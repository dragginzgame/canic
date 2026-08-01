//! Module: api::component_auth
//!
//! Responsibility: authenticate active Components at root-owned auth endpoints.
//! Does not own: Component Registry state, proof state, or attestation policy.
//! Boundary: resolves protected caller authority, then delegates to core auth workflow.

use async_trait::async_trait;
use canic_core::{
    access::{
        AccessError,
        expr::{AccessContext, AsyncAccessPredicate},
    },
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
    ids::{ComponentBinding, ManagedCanisterBinding},
};

///
/// ActiveComponentMemberPredicate
///
/// Endpoint predicate backed by exact active Component Registry membership.
///

pub struct ActiveComponentMemberPredicate;

#[async_trait]
impl AsyncAccessPredicate for ActiveComponentMemberPredicate {
    async fn eval(&self, ctx: &AccessContext) -> Result<(), AccessError> {
        active_component_member(ctx.transport_caller())
            .map(|_| ())
            .map_err(|_| {
                AccessError::Denied(format!(
                    "caller '{}' is not an active Component Registry member",
                    ctx.transport_caller()
                ))
            })
    }

    fn name(&self) -> &'static str {
        "caller_is_active_component_member"
    }
}

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
    let caller = IcOps::msg_caller();
    match active_component_member(caller)? {
        ManagedCanisterBinding::Component(binding) => Ok(binding),
        ManagedCanisterBinding::ComponentChild(_) => Err(Error::forbidden(format!(
            "caller {caller} is a Component Child, not a top-level Component"
        ))),
    }
}

fn active_component_member(caller: candid::Principal) -> Result<ManagedCanisterBinding, Error> {
    let activation = FleetActivationApi::status()?;
    if activation.phase != FleetActivationPhase::Active {
        return Err(Error::unavailable(
            "active Component membership requires an Active Fleet Subnet Root",
        ));
    }
    crate::workflow::component_registry::active_component_member(caller).map_err(Into::into)
}
