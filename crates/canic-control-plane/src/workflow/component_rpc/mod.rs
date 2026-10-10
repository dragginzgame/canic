//! Module: workflow::component_rpc
//!
//! Responsibility: resolve and dispatch root capabilities from protected Component authority.
//! Does not own: endpoint predicates, capability replay, or Component Registry persistence.
//! Boundary: binds one admitted caller/request to exact authority before core orchestration.

mod lifecycle;

use canic_contracts::{
    diagnostics::codes,
    dto::{
        capability::{RootCapabilityEnvelopeV1, RootCapabilityResponseV1},
        error::Error,
        rpc::{CreateCanisterParent, RecycleCanisterRequest, Request},
    },
    ids::ManagedCanisterBinding,
};
use canic_core::{
    api::rpc::RpcApi,
    control_plane_support::{
        error::InternalError,
        ops::ic::IcOps,
        workflow::rpc::{
            RootCapabilityAuthority, RootCapabilityCallerAuthority, RootCapabilityMemberAuthority,
            RootCapabilityParentAuthority,
        },
    },
};

/// Resolve protected request authority and dispatch one root capability.
pub async fn response_capability_v1_root(
    envelope: RootCapabilityEnvelopeV1,
) -> Result<RootCapabilityResponseV1, Error> {
    let authority = root_capability_authority(
        IcOps::msg_caller(),
        &envelope.capability,
        envelope.metadata.request_id,
    )?;
    RpcApi::response_capability_v1_root(
        envelope,
        authority,
        &lifecycle::COMPONENT_CHILD_LIFECYCLE_EXECUTOR,
    )
    .await
}

fn root_capability_authority(
    caller: candid::Principal,
    request: &Request,
    operation_id: [u8; 32],
) -> Result<RootCapabilityAuthority, Error> {
    let root = IcOps::canister_self();
    let caller = caller_authority(caller, root, request)?;
    let authority = RootCapabilityAuthority::new(caller.clone());

    match request {
        Request::AllocatePlacementChild(request) | Request::CreateCanister(request) => Ok(
            authority.with_provision_parent(resolve_provision_parent(&caller, &request.parent)?),
        ),
        Request::RecycleCanister(request) => {
            recycle_target_authority(caller, authority, request, operation_id)
        }
        Request::AcknowledgePlacementReceipt(_) | Request::Cycles(_) => Ok(authority),
    }
}

fn recycle_target_authority(
    caller: RootCapabilityCallerAuthority,
    authority: RootCapabilityAuthority,
    request: &RecycleCanisterRequest,
    operation_id: [u8; 32],
) -> Result<RootCapabilityAuthority, Error> {
    use crate::ops::component_registry::ComponentRegistryOps;

    let RootCapabilityCallerAuthority::ComponentMember(member) = &caller else {
        return Err(Error::from_registered(codes::AUTHORITY_UNAUTHORIZED));
    };
    if let Some(removal) = ComponentRegistryOps::subtree_removal(member.component(), operation_id)?
    {
        if removal.target_allocation_operation_id != request.allocation_operation_id {
            return Err(Error::from_registered(codes::AUTHORITY_CONFLICT));
        }
        if removal.target_canister_id != request.canister_pid
            || removal.target_parent_canister_id != member.canister_id()
        {
            return Err(Error::from_registered(codes::AUTHORITY_UNAUTHORIZED));
        }
        return Ok(authority.with_recovery_target(
            removal.target_canister_id,
            removal.target_parent_canister_id,
        ));
    }
    if ComponentRegistryOps::child_allocation_operation_id(
        member.component(),
        request.canister_pid,
    )? != Some(request.allocation_operation_id)
    {
        return Err(Error::from_registered(codes::AUTHORITY_CONFLICT));
    }
    resolve_active_member(request.canister_pid).map(|target| authority.with_target(target))
}

fn caller_authority(
    caller: candid::Principal,
    root: candid::Principal,
    request: &Request,
) -> Result<RootCapabilityCallerAuthority, Error> {
    if caller == root {
        super::component_auth::require_active_fleet_subnet_root()?;
        return Ok(RootCapabilityCallerAuthority::FleetSubnetRoot { canister_id: root });
    }
    let registered = super::component_registry::registered_component_member_authority(caller)
        .map_err(InternalError::from)
        .map_err(Error::from)?;
    // Membership publication can complete while the enclosing Root bootstrap still
    // waits for application initialization. Keep that interval on bounded bootstrap authority.
    let root_prepared = canic_core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::status()?
        .phase == canic_contracts::dto::fleet_activation::FleetActivationPhase::Prepared;
    let member = match registered.lifecycle {
        canic_contracts::dto::component_registry::ComponentLifecycleStatus::Prepared
        | canic_contracts::dto::component_registry::ComponentLifecycleStatus::Active
            if root_prepared
                && matches!(
                    request,
                    Request::AllocatePlacementChild(_) | Request::Cycles(_)
                ) =>
        {
            super::component_auth::require_prepared_fleet_subnet_root()?;
            RootCapabilityMemberAuthority::try_from_prepared_member(
                registered.binding,
                registered.registry,
            )
        }
        canic_contracts::dto::component_registry::ComponentLifecycleStatus::Active => {
            super::component_auth::require_active_fleet_subnet_root()?;
            RootCapabilityMemberAuthority::try_from_active_member(
                registered.binding,
                registered.registry,
            )
        }
        _ => Err(InternalError::public(codes::AUTHORITY_UNAUTHORIZED)),
    }
    .map_err(Error::from)?;
    Ok(RootCapabilityCallerAuthority::ComponentMember(member))
}

/// Resolve membership after the enclosing request has established active-root authority.
fn resolve_active_member(caller: candid::Principal) -> Result<ManagedCanisterBinding, Error> {
    super::component_registry::active_component_member(caller)
        .map_err(InternalError::from)
        .map_err(Into::into)
}

fn resolve_provision_parent(
    caller: &RootCapabilityCallerAuthority,
    selector: &CreateCanisterParent,
) -> Result<RootCapabilityParentAuthority, Error> {
    if !matches!(selector, CreateCanisterParent::ThisCanister) {
        return Err(Error::from_registered(
            canic_contracts::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    match caller {
        RootCapabilityCallerAuthority::FleetSubnetRoot { .. } => Err(Error::from_registered(
            canic_contracts::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        )),
        RootCapabilityCallerAuthority::ComponentMember(member) => Ok(member.clone().into()),
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use canic_contracts::ids::CanisterRole;

    fn p(byte: u8) -> candid::Principal {
        candid::Principal::from_slice(&[byte; 29])
    }

    #[test]
    fn structural_provision_rejects_fleet_subnet_root_as_application_parent() {
        let caller = RootCapabilityCallerAuthority::FleetSubnetRoot { canister_id: p(1) };

        let error = resolve_provision_parent(&caller, &CreateCanisterParent::ThisCanister)
            .expect_err("infrastructure root cannot own an application child");

        assert_eq!(
            error.code(),
            canic_contracts::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );
    }

    #[test]
    fn structural_provision_rejects_selector_based_parent_resolution() {
        let caller = RootCapabilityCallerAuthority::FleetSubnetRoot { canister_id: p(1) };
        let selectors = [
            CreateCanisterParent::Root,
            CreateCanisterParent::Parent,
            CreateCanisterParent::Canister(p(2)),
            CreateCanisterParent::Directory(CanisterRole::from("project_hub")),
        ];

        for selector in selectors {
            let error = resolve_provision_parent(&caller, &selector)
                .expect_err("non-structural parent selector must reject");
            assert_eq!(
                error.code(),
                canic_contracts::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
            );
        }
    }
}
