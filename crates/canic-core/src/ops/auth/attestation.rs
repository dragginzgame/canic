use super::{AuthOps, crypto, keys, verify};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{
        AttestationKeySet, InternalInvocationProofPayloadV1, RoleAttestation,
        SignedInternalInvocationProofV1, SignedRoleAttestation,
    },
    ops::{
        auth::{
            AuthOpsError, AuthSignatureError, AuthValidationError,
            InternalInvocationProofVerificationInput,
        },
        ic::{IcOps, ecdsa::EcdsaOps},
        storage::auth::AuthStateOps,
    },
};

impl AuthOps {
    pub async fn attestation_key_set() -> Result<AttestationKeySet, InternalError> {
        let root_pid = IcOps::canister_self();
        let now_secs = IcOps::now_secs();
        let key_name = keys::attestation_key_name()?;
        keys::ensure_attestation_key_cached(&key_name, root_pid, now_secs).await?;

        Ok(AttestationKeySet {
            root_pid,
            generated_at: now_secs,
            keys: keys::attestation_keys_sorted(&key_name),
        })
    }

    pub fn replace_attestation_key_set(key_set: AttestationKeySet) {
        AuthStateOps::set_attestation_key_set(key_set);
    }

    pub(crate) fn verify_role_attestation_cached(
        attestation: &SignedRoleAttestation,
        caller: Principal,
        self_pid: Principal,
        verifier_subnet: Option<Principal>,
        now_ns: u64,
        min_accepted_epoch: u64,
    ) -> Result<RoleAttestation, AuthOpsError> {
        if attestation.signature.is_empty() {
            return Err(AuthSignatureError::AttestationSignatureUnavailable.into());
        }

        let key_name = keys::attestation_key_name()
            .map_err(|err| AuthValidationError::Auth(err.to_string()))?;
        let key = AuthStateOps::attestation_public_key(attestation.key_id, &key_name).ok_or(
            AuthValidationError::AttestationUnknownKeyId {
                key_id: attestation.key_id,
            },
        )?;
        let now_secs = now_ns / 1_000_000_000;
        verify::verify_attestation_key_validity(&key, now_secs)?;

        let public_key = key.public_key;
        let hash = crypto::role_attestation_hash(&attestation.payload)
            .map_err(|err| AuthSignatureError::AttestationSignatureInvalid(err.to_string()))?;
        EcdsaOps::verify_signature(&public_key, hash, &attestation.signature)
            .map_err(|err| AuthSignatureError::AttestationSignatureInvalid(err.to_string()))?;

        verify::verify_role_attestation_claims(
            &attestation.payload,
            caller,
            self_pid,
            verifier_subnet,
            now_ns,
            min_accepted_epoch,
        )?;

        Ok(attestation.payload.clone())
    }

    pub(crate) fn verify_internal_invocation_proof_cached(
        proof: &SignedInternalInvocationProofV1,
        input: InternalInvocationProofVerificationInput<'_>,
    ) -> Result<InternalInvocationProofPayloadV1, AuthOpsError> {
        if proof.signature.is_empty() {
            return Err(AuthSignatureError::AttestationSignatureUnavailable.into());
        }

        let key_name = keys::attestation_key_name()
            .map_err(|err| AuthValidationError::Auth(err.to_string()))?;
        let key = AuthStateOps::attestation_public_key(proof.key_id, &key_name).ok_or(
            AuthValidationError::AttestationUnknownKeyId {
                key_id: proof.key_id,
            },
        )?;
        verify::verify_attestation_key_validity(&key, input.now_ns / 1_000_000_000)?;

        let public_key = key.public_key;
        let hash = crypto::internal_invocation_proof_hash(&proof.payload)
            .map_err(|err| AuthSignatureError::AttestationSignatureInvalid(err.to_string()))?;
        EcdsaOps::verify_signature(&public_key, hash, &proof.signature)
            .map_err(|err| AuthSignatureError::AttestationSignatureInvalid(err.to_string()))?;

        verify::verify_internal_invocation_proof_claims(&proof.payload, input)?;

        Ok(proof.payload.clone())
    }
}
