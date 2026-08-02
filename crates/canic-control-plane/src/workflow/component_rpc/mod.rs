//! Module: workflow::component_rpc
//!
//! Responsibility: resolve and dispatch root capabilities from protected Component authority.
//! Does not own: endpoint predicates, capability replay, or Component Registry persistence.
//! Boundary: binds one admitted caller/request to exact authority before core orchestration.

mod lifecycle;

use canic_core::{
    api::rpc::RpcApi,
    control_plane_support::{
        ops::ic::IcOps,
        workflow::rpc::{
            RootCapabilityAuthority, RootCapabilityCallerAuthority, RootCapabilityMemberAuthority,
            RootCapabilityParentAuthority,
        },
    },
    dto::{
        capability::{RootCapabilityEnvelopeV1, RootCapabilityResponseV1},
        error::{Error, ErrorCode},
        rpc::{CreateCanisterParent, Request},
    },
    ids::ManagedCanisterBinding,
};

/// Resolve protected request authority and dispatch one root capability.
pub async fn response_capability_v1_root(
    envelope: RootCapabilityEnvelopeV1,
) -> Result<RootCapabilityResponseV1, Error> {
    let authority = root_capability_authority(IcOps::msg_caller(), &envelope.capability)?;
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
) -> Result<RootCapabilityAuthority, Error> {
    super::component_auth::require_active_fleet_subnet_root()?;
    let root = IcOps::canister_self();
    let caller = caller_authority(caller, root)?;
    let authority = RootCapabilityAuthority::new(caller.clone());

    match request {
        Request::AllocatePlacementChild(request) | Request::CreateCanister(request) => Ok(
            authority.with_provision_parent(resolve_provision_parent(&caller, &request.parent)?),
        ),
        Request::UpgradeCanister(request) => {
            Ok(authority.with_target(resolve_active_member(request.canister_pid)?))
        }
        Request::RecycleCanister(request) => {
            optional_active_recycle_target(authority, request.canister_pid)
        }
        Request::AcknowledgePlacementReceipt(_) | Request::Cycles(_) => Ok(authority),
    }
}

fn optional_active_recycle_target(
    authority: RootCapabilityAuthority,
    target: candid::Principal,
) -> Result<RootCapabilityAuthority, Error> {
    match resolve_active_member(target) {
        Ok(target) => Ok(authority.with_target(target)),
        Err(error) if error.code == ErrorCode::Forbidden => Ok(authority),
        Err(error) => Err(error),
    }
}

fn caller_authority(
    caller: candid::Principal,
    root: candid::Principal,
) -> Result<RootCapabilityCallerAuthority, Error> {
    if caller == root {
        return Ok(RootCapabilityCallerAuthority::FleetSubnetRoot { canister_id: root });
    }
    let active = super::component_registry::active_component_member_authority(caller)
        .map_err(Error::from)?;
    let member =
        RootCapabilityMemberAuthority::try_from_active_member(active.binding, active.registry)
            .map_err(Error::from)?;
    Ok(RootCapabilityCallerAuthority::ComponentMember(member))
}

/// Resolve membership after the enclosing request has established active-root authority.
fn resolve_active_member(caller: candid::Principal) -> Result<ManagedCanisterBinding, Error> {
    super::component_registry::active_component_member(caller).map_err(Into::into)
}

fn resolve_provision_parent(
    caller: &RootCapabilityCallerAuthority,
    selector: &CreateCanisterParent,
) -> Result<RootCapabilityParentAuthority, Error> {
    if !matches!(selector, CreateCanisterParent::ThisCanister) {
        return Err(Error::forbidden(
            "root structural provision requires parent=ThisCanister",
        ));
    }
    match caller {
        RootCapabilityCallerAuthority::FleetSubnetRoot { .. } => Err(Error::forbidden(
            "Fleet Subnet Root cannot provision application children through Component RPC",
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
    use canic_core::ids::CanisterRole;

    fn p(byte: u8) -> candid::Principal {
        candid::Principal::from_slice(&[byte; 29])
    }

    #[test]
    fn structural_provision_rejects_fleet_subnet_root_as_application_parent() {
        let caller = RootCapabilityCallerAuthority::FleetSubnetRoot { canister_id: p(1) };

        let error = resolve_provision_parent(&caller, &CreateCanisterParent::ThisCanister)
            .expect_err("infrastructure root cannot own an application child");

        assert_eq!(error.code, ErrorCode::Forbidden);
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
            assert_eq!(error.code, ErrorCode::Forbidden);
        }
    }
}
