use crate::{
    cdk::types::Principal,
    dto::{
        capability::{
            CapabilityProof, CapabilityProofBlob, DelegatedGrantProof, RoleAttestationProof,
        },
        error::Error,
        rpc::{Request, RequestFamily},
    },
    ops::{
        ic::IcOps, storage::children::CanisterChildrenOps,
        storage::registry::subnet::SubnetRegistryOps,
    },
};
use candid::{decode_one, encode_one};
use std::convert::TryFrom;

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

    if capability.family() == RequestFamily::RequestCycles {
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

// --- Wire Encoding ------------------------------------------------------

// Encode the full role-attestation proof into the compact shared wire blob.
pub(super) fn encode_role_attestation_blob(
    proof: &RoleAttestationProof,
) -> Result<CapabilityProofBlob, Error> {
    Ok(CapabilityProofBlob {
        proof_version: proof.proof_version,
        capability_hash: proof.capability_hash,
        payload: encode_one(proof).map_err(|err| {
            Error::internal(format!("failed to encode role attestation proof: {err}"))
        })?,
    })
}

// Decode a role-attestation wire blob back into its concrete proof payload.
pub(super) fn decode_role_attestation_blob(
    blob: &CapabilityProofBlob,
) -> Result<RoleAttestationProof, Error> {
    let proof: RoleAttestationProof = decode_one(&blob.payload)
        .map_err(|err| Error::invalid(format!("failed to decode role attestation proof: {err}")))?;

    if proof.proof_version != blob.proof_version {
        return Err(Error::invalid(
            "role attestation proof_version does not match wire header",
        ));
    }
    if proof.capability_hash != blob.capability_hash {
        return Err(Error::invalid(
            "role attestation capability_hash does not match wire header",
        ));
    }

    Ok(proof)
}

// Encode the full delegated-grant proof into the compact shared wire blob.
pub(super) fn encode_delegated_grant_blob(
    proof: &DelegatedGrantProof,
) -> Result<CapabilityProofBlob, Error> {
    Ok(CapabilityProofBlob {
        proof_version: proof.proof_version,
        capability_hash: proof.capability_hash,
        payload: encode_one(proof).map_err(|err| {
            Error::internal(format!("failed to encode delegated grant proof: {err}"))
        })?,
    })
}

// Decode a delegated-grant wire blob back into its concrete proof payload.
pub(super) fn decode_delegated_grant_blob(
    blob: &CapabilityProofBlob,
) -> Result<DelegatedGrantProof, Error> {
    let proof: DelegatedGrantProof = decode_one(&blob.payload)
        .map_err(|err| Error::invalid(format!("failed to decode delegated grant proof: {err}")))?;

    if proof.proof_version != blob.proof_version {
        return Err(Error::invalid(
            "delegated grant proof_version does not match wire header",
        ));
    }
    if proof.capability_hash != blob.capability_hash {
        return Err(Error::invalid(
            "delegated grant capability_hash does not match wire header",
        ));
    }

    Ok(proof)
}

impl TryFrom<RoleAttestationProof> for CapabilityProof {
    type Error = Error;

    fn try_from(value: RoleAttestationProof) -> Result<Self, Self::Error> {
        Ok(Self::RoleAttestation(encode_role_attestation_blob(&value)?))
    }
}

impl TryFrom<DelegatedGrantProof> for CapabilityProof {
    type Error = Error;

    fn try_from(value: DelegatedGrantProof) -> Result<Self, Self::Error> {
        Ok(Self::DelegatedGrant(encode_delegated_grant_blob(&value)?))
    }
}
