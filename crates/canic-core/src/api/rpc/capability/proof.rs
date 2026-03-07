use crate::{
    cdk::types::Principal,
    dto::{capability::CapabilityProof, error::Error, rpc::Request},
    ops::{ic::IcOps, storage::registry::subnet::SubnetRegistryOps},
};

pub(super) async fn verify_root_capability_proof(
    capability: &Request,
    capability_version: u16,
    proof: &CapabilityProof,
) -> Result<(), Error> {
    let target_canister = IcOps::canister_self();

    match proof {
        CapabilityProof::Structural => verify_root_structural_proof(capability),
        CapabilityProof::RoleAttestation(proof) => {
            verify_capability_hash_binding(
                target_canister,
                capability_version,
                capability,
                proof.capability_hash,
            )?;

            crate::api::auth::DelegationApi::verify_role_attestation(&proof.attestation, 0).await
        }
        CapabilityProof::DelegatedGrant(proof) => {
            verify_capability_hash_binding(
                target_canister,
                capability_version,
                capability,
                proof.capability_hash,
            )?;
            super::verify_delegated_grant_hash_binding(proof)?;
            super::verify_root_delegated_grant_proof(
                capability,
                proof,
                IcOps::msg_caller(),
                target_canister,
                IcOps::now_secs(),
            )
        }
    }
}

fn verify_root_structural_proof(capability: &Request) -> Result<(), Error> {
    let caller = IcOps::msg_caller();

    if SubnetRegistryOps::get(caller).is_none() {
        return Err(Error::forbidden(
            "structural proof requires caller to be registered in subnet registry",
        ));
    }

    match capability {
        Request::Cycles(_) => Ok(()),
        Request::UpgradeCanister(req) => {
            let target = SubnetRegistryOps::get(req.canister_pid).ok_or_else(|| {
                Error::forbidden("structural proof requires registered upgrade target")
            })?;
            if target.parent_pid != Some(caller) {
                return Err(Error::forbidden(
                    "structural proof requires upgrade target to be a direct child of caller",
                ));
            }
            Ok(())
        }
        _ => Err(Error::forbidden(
            "structural proof is only supported for root cycles and upgrade capabilities",
        )),
    }
}

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
