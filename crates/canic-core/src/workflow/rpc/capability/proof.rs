//! Module: workflow::rpc::capability::proof
//!
//! Responsibility: verify structural capability proof constraints.
//! Does not own: envelope validation, metrics, request dispatch, or replay metadata.
//! Boundary: checks caller topology and canonical capability hash bindings.

use crate::{
    cdk::types::Principal,
    dto::{
        error::Error,
        rpc::{CreateCanisterParent, Request},
    },
    ops::{
        ic::IcOps, storage::children::CanisterChildrenOps,
        storage::registry::subnet::SubnetRegistryOps,
    },
};

/// verify_root_structural_proof
///
/// Verify structural proof constraints for capability families that allow it.
pub(super) fn verify_root_structural_proof(capability: &Request) -> Result<(), Error> {
    let caller = IcOps::msg_caller();

    if !SubnetRegistryOps::is_registered(caller) {
        return Err(Error::forbidden(
            "structural proof requires caller to be registered in subnet registry",
        ));
    }

    match capability {
        Request::Cycles(_) => Ok(()),
        Request::CreateCanister(request) => verify_root_structural_create(request),
        Request::UpgradeCanister(request) => {
            verify_root_structural_child_target(caller, request.canister_pid, "upgrade")
        }
        Request::RecycleCanister(request) => {
            verify_root_structural_child_target(caller, request.canister_pid, "recycle")
        }
    }
}

fn verify_root_structural_create(
    request: &crate::dto::rpc::CreateCanisterRequest,
) -> Result<(), Error> {
    if matches!(&request.parent, CreateCanisterParent::ThisCanister) {
        return Ok(());
    }

    Err(Error::forbidden(
        "structural provision proof requires parent=ThisCanister",
    ))
}

fn verify_root_structural_child_target(
    caller: Principal,
    target_pid: Principal,
    operation: &str,
) -> Result<(), Error> {
    let (_, target_parent) = SubnetRegistryOps::role_parent(target_pid).ok_or_else(|| {
        Error::forbidden(format!(
            "structural proof requires registered {operation} target"
        ))
    })?;
    if target_parent != Some(caller) {
        return Err(Error::forbidden(format!(
            "structural proof requires {operation} target to be a direct child of caller"
        )));
    }
    Ok(())
}

/// verify_nonroot_structural_cycles_proof
///
/// Verify that a structural cycles request came from a cached direct child.
pub(super) fn verify_nonroot_structural_cycles_proof() -> Result<(), Error> {
    let caller = IcOps::msg_caller();

    if !CanisterChildrenOps::contains_pid(&caller) {
        return Err(Error::forbidden(
            "structural proof requires caller to be a direct child of receiver",
        ));
    }

    Ok(())
}

/// verify_capability_hash_binding
///
/// Ensure the proof hash matches canonical capability payload bytes.
#[cfg(test)]
pub(super) fn verify_capability_hash_binding(
    target_canister: Principal,
    capability_version: u16,
    capability: &Request,
    capability_hash: [u8; 32],
) -> Result<(), Error> {
    let expected = crate::workflow::rpc::capability::root_capability_hash(
        target_canister,
        capability_version,
        capability,
    )?;
    if capability_hash != expected {
        return Err(Error::invalid(
            "capability_hash does not match capability payload",
        ));
    }

    Ok(())
}
