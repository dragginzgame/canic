use crate::{
    cdk::types::Principal,
    dto::{
        error::Error,
        rpc::{Request, RequestFamily},
    },
    ops::{ic::IcOps, storage::registry::subnet::SubnetRegistryOps},
};

/// verify_root_structural_proof
///
/// Verify structural proof constraints for capability families that allow it.
pub(super) fn verify_root_structural_proof(capability: &Request) -> Result<(), Error> {
    let caller = IcOps::msg_caller();

    if SubnetRegistryOps::get(caller).is_none() {
        return Err(Error::forbidden(
            "structural proof requires caller to be registered in subnet registry",
        ));
    }

    if capability.family() == RequestFamily::MintCycles {
        return Ok(());
    }

    if let Some(request) = capability.upgrade_request() {
        let target = SubnetRegistryOps::get(request.canister_pid).ok_or_else(|| {
            Error::forbidden("structural proof requires registered upgrade target")
        })?;
        if target.parent_pid != Some(caller) {
            return Err(Error::forbidden(
                "structural proof requires upgrade target to be a direct child of caller",
            ));
        }
        return Ok(());
    }

    Err(Error::forbidden(
        "structural proof is only supported for root cycles and upgrade capabilities",
    ))
}

/// verify_capability_hash_binding
///
/// Ensure the proof hash matches canonical capability payload bytes.
pub(super) fn verify_capability_hash_binding(
    target_canister: Principal,
    capability_version: u16,
    capability: &Request,
    capability_hash: [u8; 32],
) -> Result<(), Error> {
    let expected = super::root_capability_hash(target_canister, capability_version, capability)?;
    if capability_hash != expected {
        return Err(Error::invalid(
            "capability_hash does not match capability payload",
        ));
    }

    Ok(())
}
