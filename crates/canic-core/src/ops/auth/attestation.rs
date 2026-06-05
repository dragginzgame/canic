use super::{
    AuthOps, PreparedInternalInvocationProofSignature, PreparedRoleAttestationSignature,
    ROLE_ATTESTATION_KEY_ID_V1, crypto, delegated::canonical::key_name_hash, keys, verify,
};
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
        cost_guard::CostGuardPermit,
        ic::{IcOps, ecdsa::EcdsaOps},
        replay::model::{EcdsaPurpose, ExternalEffectDescriptor},
        storage::auth::AuthStateOps,
    },
};

impl AuthOps {
    /// Prepare a role attestation payload before root ECDSA signing.
    pub(crate) async fn prepare_role_attestation_signature(
        payload: RoleAttestation,
    ) -> Result<PreparedRoleAttestationSignature, InternalError> {
        let key_name = keys::attestation_key_name()?;
        keys::ensure_attestation_key_cached(&key_name, IcOps::canister_self(), IcOps::now_secs())
            .await?;
        let hash = crypto::role_attestation_hash(&payload)?;
        Ok(PreparedRoleAttestationSignature {
            payload,
            message_hash: hash,
            key_name,
            derivation_path: keys::attestation_derivation_path(),
        })
    }

    /// Sign a prepared role attestation payload using the attestation domain.
    pub(crate) async fn sign_prepared_role_attestation(
        _permit: &CostGuardPermit,
        prepared: PreparedRoleAttestationSignature,
    ) -> Result<SignedRoleAttestation, InternalError> {
        let signature = EcdsaOps::sign_bytes(
            &prepared.key_name,
            prepared.derivation_path,
            prepared.message_hash,
        )
        .await?;

        Ok(SignedRoleAttestation {
            payload: prepared.payload,
            signature,
            key_id: ROLE_ATTESTATION_KEY_ID_V1,
        })
    }

    /// Prepare a method-scoped internal invocation proof before root ECDSA signing.
    pub(crate) async fn prepare_internal_invocation_proof_signature(
        payload: InternalInvocationProofPayloadV1,
    ) -> Result<PreparedInternalInvocationProofSignature, InternalError> {
        let key_name = keys::attestation_key_name()?;
        keys::ensure_attestation_key_cached(&key_name, IcOps::canister_self(), IcOps::now_secs())
            .await?;
        let hash = crypto::internal_invocation_proof_hash(&payload)?;
        Ok(PreparedInternalInvocationProofSignature {
            payload,
            message_hash: hash,
            key_name,
            derivation_path: keys::attestation_derivation_path(),
        })
    }

    /// Sign a prepared method-scoped internal invocation proof using the 0.40 proof domain.
    pub(crate) async fn sign_prepared_internal_invocation_proof(
        _permit: &CostGuardPermit,
        prepared: PreparedInternalInvocationProofSignature,
    ) -> Result<SignedInternalInvocationProofV1, InternalError> {
        let signature = EcdsaOps::sign_bytes(
            &prepared.key_name,
            prepared.derivation_path,
            prepared.message_hash,
        )
        .await?;

        Ok(SignedInternalInvocationProofV1 {
            payload: prepared.payload,
            signature,
            key_id: ROLE_ATTESTATION_KEY_ID_V1,
        })
    }

    /// Describe the root ECDSA effect for a prepared role attestation signature.
    pub(crate) fn role_attestation_signing_effect(
        prepared: &PreparedRoleAttestationSignature,
    ) -> ExternalEffectDescriptor {
        ExternalEffectDescriptor::ThresholdEcdsaSign {
            key_id_hash: key_name_hash(&prepared.key_name),
            purpose: EcdsaPurpose::RoleAttestation,
            message_hash: prepared.message_hash,
        }
    }

    /// Describe the root ECDSA effect for a prepared internal invocation proof signature.
    pub(crate) fn internal_invocation_proof_signing_effect(
        prepared: &PreparedInternalInvocationProofSignature,
    ) -> ExternalEffectDescriptor {
        ExternalEffectDescriptor::ThresholdEcdsaSign {
            key_id_hash: key_name_hash(&prepared.key_name),
            purpose: EcdsaPurpose::InternalInvocationProof,
            message_hash: prepared.message_hash,
        }
    }

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
        now_secs: u64,
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
            now_secs,
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
        verify::verify_attestation_key_validity(&key, input.now_secs)?;

        let public_key = key.public_key;
        let hash = crypto::internal_invocation_proof_hash(&proof.payload)
            .map_err(|err| AuthSignatureError::AttestationSignatureInvalid(err.to_string()))?;
        EcdsaOps::verify_signature(&public_key, hash, &proof.signature)
            .map_err(|err| AuthSignatureError::AttestationSignatureInvalid(err.to_string()))?;

        verify::verify_internal_invocation_proof_claims(&proof.payload, input)?;

        Ok(proof.payload.clone())
    }
}
