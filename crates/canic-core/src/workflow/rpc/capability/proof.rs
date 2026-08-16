//! Module: workflow::rpc::capability::proof
//!
//! Responsibility: verify structural capability proof constraints.
//! Does not own: envelope validation, metrics, request dispatch, or replay metadata.
//! Boundary: checks caller topology against protected root capability authority.

use crate::{
    cdk::types::Principal,
    dto::{error::Error, rpc::CreateCanisterParent},
    ops::{ic::IcOps, storage::children::CanisterChildrenOps},
    workflow::rpc::{RootCapabilityAuthority, request::handler::capability::RootCapability},
};

/// verify_root_structural_proof
///
/// Verify structural proof constraints for capability families that allow it.
pub(super) fn verify_root_structural_proof(
    capability: &RootCapability,
    authority: &RootCapabilityAuthority,
) -> Result<(), Error> {
    let caller = IcOps::msg_caller();
    if authority.caller_canister_id() != caller {
        return Err(Error::from_registered(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }

    match capability {
        RootCapability::AcknowledgePlacementReceipt(_) | RootCapability::RequestCycles(_) => {
            require_no_scoped_authority(authority)
        }
        RootCapability::AllocatePlacementChild(request)
        | RootCapability::ProvisionCanister(request) => {
            verify_root_structural_create(request, authority)
        }
        RootCapability::RecycleCanister(request) => {
            verify_root_structural_child_target(caller, request.canister_pid, "recycle", authority)
        }
    }
}

fn verify_root_structural_create(
    request: &crate::dto::rpc::CreateCanisterRequest,
    authority: &RootCapabilityAuthority,
) -> Result<(), Error> {
    if !matches!(&request.parent, CreateCanisterParent::ThisCanister) {
        return Err(Error::from_registered(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    require_no_target_authority(authority)?;
    if authority.provision_parent_canister_id() != Some(authority.caller_canister_id()) {
        return Err(Error::from_registered(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    Ok(())
}

fn verify_root_structural_child_target(
    caller: Principal,
    target_pid: Principal,
    _operation: &str,
    authority: &RootCapabilityAuthority,
) -> Result<(), Error> {
    require_no_provision_parent_authority(authority)?;
    if authority.target_canister_id() != Some(target_pid) {
        return Err(Error::from_registered(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    if authority.target_parent_canister_id() != Some(caller) {
        return Err(Error::from_registered(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    Ok(())
}

fn require_no_scoped_authority(authority: &RootCapabilityAuthority) -> Result<(), Error> {
    require_no_target_authority(authority)?;
    require_no_provision_parent_authority(authority)
}

const fn require_no_target_authority(authority: &RootCapabilityAuthority) -> Result<(), Error> {
    if authority.has_target() {
        return Err(Error::from_registered(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    Ok(())
}

const fn require_no_provision_parent_authority(
    authority: &RootCapabilityAuthority,
) -> Result<(), Error> {
    if authority.has_provision_parent() {
        return Err(Error::from_registered(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    Ok(())
}

/// verify_nonroot_structural_cycles_proof
///
/// Verify that a structural cycles request came from a cached direct child.
pub(super) fn verify_nonroot_structural_cycles_proof() -> Result<(), Error> {
    let caller = IcOps::msg_caller();

    if !CanisterChildrenOps::contains_pid(&caller) {
        return Err(Error::from_registered(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }

    Ok(())
}
