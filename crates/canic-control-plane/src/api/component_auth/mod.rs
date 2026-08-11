//! Module: api::component_auth
//!
//! Responsibility: expose active Component membership to root auth and application endpoints.
//! Does not own: Component Registry state, proof state, or attestation policy.
//! Boundary: resolves protected local Registry authority before workflow delegation.

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
/// RootComponentMembershipApi
///
/// Read-only application facade over one Fleet Subnet Root's active Component Registry.
///

pub struct RootComponentMembershipApi;

impl RootComponentMembershipApi {
    /// Resolve an exact active local Component Registry member by Canister principal.
    ///
    /// Root application endpoints may use this to derive local topology authority for a
    /// transport caller. The caller-facing endpoint must independently authorize access to
    /// the lookup, and cross-root callers require the Fleet-service peer authority path.
    pub fn active_member(subject: candid::Principal) -> Result<ManagedCanisterBinding, Error> {
        crate::workflow::component_auth::active_component_member(subject)
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
    RootComponentMembershipApi::active_member(IcOps::msg_caller())
}
