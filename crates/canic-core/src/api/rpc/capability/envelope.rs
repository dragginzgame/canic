use crate::dto::{
    capability::{CAPABILITY_VERSION_V1, CapabilityProof, CapabilityService, PROOF_VERSION_V1},
    error::Error,
};

pub(super) fn validate_root_capability_envelope(
    service: CapabilityService,
    capability_version: u16,
    proof: &CapabilityProof,
) -> Result<(), Error> {
    if service != CapabilityService::Root {
        return Err(Error::invalid(
            "capability envelope service must be Root for root dispatch",
        ));
    }

    if capability_version != CAPABILITY_VERSION_V1 {
        return Err(Error::invalid(format!(
            "unsupported capability_version: {capability_version}",
        )));
    }

    match proof {
        CapabilityProof::Structural => Ok(()),
        CapabilityProof::RoleAttestation(proof) => {
            if proof.proof_version != PROOF_VERSION_V1 {
                return Err(Error::invalid(format!(
                    "unsupported role attestation proof_version: {}",
                    proof.proof_version
                )));
            }
            Ok(())
        }
        CapabilityProof::DelegatedGrant(proof) => {
            if proof.proof_version != PROOF_VERSION_V1 {
                return Err(Error::invalid(format!(
                    "unsupported delegated grant proof_version: {}",
                    proof.proof_version
                )));
            }
            Ok(())
        }
    }
}
