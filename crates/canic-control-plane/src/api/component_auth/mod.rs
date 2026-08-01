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
    api::auth::AuthApi,
    control_plane_support::ops::ic::IcOps,
    dto::{
        auth::{
            RoleAttestationGetRequest, RoleAttestationPrepareResponse, RoleAttestationRequest,
            SignedRoleAttestation,
        },
        error::Error,
    },
    ids::ManagedCanisterBinding,
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
        crate::workflow::component_auth::active_component_member(ctx.transport_caller())
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
    /// Prepare a role attestation for the exact active Component Registry caller.
    pub fn prepare_role_attestation(
        request: RoleAttestationRequest,
    ) -> Result<RoleAttestationPrepareResponse, Error> {
        let member = active_component_caller()?;
        AuthApi::prepare_component_role_attestation_root(request, &member)
    }

    /// Retrieve caller-bound proof only while the Component remains active.
    pub fn get_role_attestation(
        request: RoleAttestationGetRequest,
    ) -> Result<SignedRoleAttestation, Error> {
        active_component_caller()?;
        AuthApi::get_role_attestation_root(request)
    }
}

fn active_component_caller() -> Result<ManagedCanisterBinding, Error> {
    crate::workflow::component_auth::active_component_member(IcOps::msg_caller())
}
