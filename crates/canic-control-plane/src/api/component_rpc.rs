//! Module: api::component_rpc
//!
//! Responsibility: expose the active-member-authenticated root capability boundary.
//! Does not own: authority resolution, capability replay, or Component Registry persistence.
//! Boundary: endpoint admission delegates immediately to control-plane workflow.

use async_trait::async_trait;
use canic_core::{
    access::{
        AccessError,
        expr::{AccessContext, AsyncAccessPredicate},
    },
    control_plane_support::ops::ic::IcOps,
    dto::{
        capability::{RootCapabilityEnvelopeV1, RootCapabilityResponseV1},
        error::Error,
    },
};

///
/// RootCapabilityCallerPredicate
///
/// Endpoint predicate admitting the root itself or one exact active Component member.
///

pub struct RootCapabilityCallerPredicate;

#[async_trait]
impl AsyncAccessPredicate for RootCapabilityCallerPredicate {
    async fn eval(&self, ctx: &AccessContext) -> Result<(), AccessError> {
        if ctx.transport_caller() == IcOps::canister_self() {
            return crate::workflow::component_auth::require_active_fleet_subnet_root_internal()
                .map_err(AccessError::Internal);
        }
        match crate::workflow::component_auth::active_component_member_for_access(
            ctx.transport_caller(),
        ) {
            Ok(_) => Ok(()),
            Err(crate::workflow::component_auth::ActiveComponentMemberError::NotActive) => {
                Err(root_capability_denial())
            }
            Err(crate::workflow::component_auth::ActiveComponentMemberError::Internal(error)) => {
                Err(AccessError::Internal(error))
            }
        }
    }

    fn name(&self) -> &'static str {
        "caller_is_root_or_active_component_member"
    }
}

///
/// ComponentRpcApi
///
/// Root capability facade backed only by protected Component Registry evidence.
///

pub struct ComponentRpcApi;

impl ComponentRpcApi {
    /// Resolve exact request authority before entering core replay and execution.
    pub async fn response_capability_v1_root(
        envelope: RootCapabilityEnvelopeV1,
    ) -> Result<RootCapabilityResponseV1, Error> {
        crate::workflow::component_rpc::response_capability_v1_root(envelope).await
    }
}

const fn root_capability_denial() -> AccessError {
    AccessError::RootOrActiveComponentRequired
}
