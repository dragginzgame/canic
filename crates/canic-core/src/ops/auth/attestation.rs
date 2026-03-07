use super::{DelegatedTokenOps, ROLE_ATTESTATION_KEY_ID_V1, crypto, keys, verify};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{AttestationKeySet, RoleAttestation, SignedRoleAttestation},
    ops::{
        auth::DelegatedTokenOpsError,
        ic::{IcOps, ecdsa::EcdsaOps},
        storage::auth::DelegationStateOps,
    },
};

impl DelegatedTokenOps {
    /// Sign a role attestation payload using the attestation domain.
    pub(crate) async fn sign_role_attestation(
        payload: RoleAttestation,
    ) -> Result<SignedRoleAttestation, InternalError> {
        let key_name = keys::attestation_key_name()?;
        keys::ensure_attestation_key_cached(&key_name, IcOps::canister_self(), IcOps::now_secs())
            .await?;
        let hash = crypto::role_attestation_hash(&payload)?;
        let signature =
            EcdsaOps::sign_bytes(&key_name, keys::attestation_derivation_path(), hash).await?;

        Ok(SignedRoleAttestation {
            payload,
            signature,
            key_id: ROLE_ATTESTATION_KEY_ID_V1,
        })
    }

    pub async fn attestation_key_set() -> Result<AttestationKeySet, InternalError> {
        let root_pid = IcOps::canister_self();
        let now_secs = IcOps::now_secs();
        let key_name = keys::attestation_key_name()?;
        keys::ensure_attestation_key_cached(&key_name, root_pid, now_secs).await?;

        Ok(AttestationKeySet {
            root_pid,
            generated_at: now_secs,
            keys: keys::attestation_keys_sorted(),
        })
    }

    pub fn replace_attestation_key_set(key_set: AttestationKeySet) {
        DelegationStateOps::set_attestation_key_set(key_set);
    }

    pub(crate) fn verify_role_attestation_cached(
        attestation: &SignedRoleAttestation,
        caller: Principal,
        self_pid: Principal,
        verifier_subnet: Option<Principal>,
        now_secs: u64,
        min_accepted_epoch: u64,
    ) -> Result<RoleAttestation, DelegatedTokenOpsError> {
        if attestation.signature.is_empty() {
            return Err(DelegatedTokenOpsError::AttestationSignatureUnavailable);
        }

        let key = DelegationStateOps::attestation_public_key(attestation.key_id).ok_or(
            DelegatedTokenOpsError::AttestationUnknownKeyId {
                key_id: attestation.key_id,
            },
        )?;
        verify::verify_attestation_key_validity(&key, now_secs)?;

        let public_key = key.public_key;
        let hash = crypto::role_attestation_hash(&attestation.payload)
            .map_err(|err| DelegatedTokenOpsError::AttestationSignatureInvalid(err.to_string()))?;
        EcdsaOps::verify_signature(&public_key, hash, &attestation.signature)
            .map_err(|err| DelegatedTokenOpsError::AttestationSignatureInvalid(err.to_string()))?;

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
}
