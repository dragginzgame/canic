use super::{INTERNAL_INVOCATION_PROOF_SIGNING_DOMAIN, ROLE_ATTESTATION_SIGNING_DOMAIN};
use crate::{
    InternalError,
    dto::auth::{InternalInvocationProofPayloadV1, RoleAttestation},
    ops::{auth::AuthValidationError, prelude::*},
};
use candid::encode_one;
use sha2::{Digest, Sha256};

pub(super) fn encode_candid<T: CandidType>(
    context: &'static str,
    value: &T,
) -> Result<Vec<u8>, InternalError> {
    encode_one(value).map_err(|err| {
        AuthValidationError::EncodeFailed {
            context,
            source: err,
        }
        .into()
    })
}

pub(super) fn role_attestation_hash(
    attestation: &RoleAttestation,
) -> Result<[u8; 32], InternalError> {
    let payload = encode_candid("role attestation", attestation)?;
    Ok(domain_separated_hash(
        ROLE_ATTESTATION_SIGNING_DOMAIN,
        payload,
    ))
}

pub(super) fn internal_invocation_proof_hash(
    proof: &InternalInvocationProofPayloadV1,
) -> Result<[u8; 32], InternalError> {
    let payload = encode_candid("internal invocation proof", proof)?;
    Ok(domain_separated_hash(
        INTERNAL_INVOCATION_PROOF_SIGNING_DOMAIN,
        payload,
    ))
}

fn domain_separated_hash(domain: &[u8], payload: Vec<u8>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(payload);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::{internal_invocation_proof_hash, role_attestation_hash};
    use crate::{cdk::types::Principal, dto::auth::*, ids::CanisterRole};

    #[test]
    fn internal_invocation_proofs_use_distinct_signing_domain() {
        let subject = Principal::from_slice(&[1]);
        let audience = Principal::from_slice(&[2]);
        let role = CanisterRole::new("project_hub");
        let role_attestation = RoleAttestation {
            subject,
            role: role.clone(),
            subnet_id: None,
            audience,
            issued_at_ns: 10,
            expires_at_ns: 20,
            epoch: 3,
        };
        let internal_proof = InternalInvocationProofPayloadV1 {
            subject,
            role,
            subnet_id: None,
            audience,
            audience_method: "system_add_project_to_user".to_string(),
            issued_at_ns: 10,
            expires_at_ns: 20,
            epoch: 3,
        };

        let role_hash = role_attestation_hash(&role_attestation).expect("role hash");
        let internal_hash = internal_invocation_proof_hash(&internal_proof).expect("internal hash");

        assert_ne!(role_hash, internal_hash);
    }
}
