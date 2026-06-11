use super::{
    AuthOps, PreparedRootRoleAttestation, SignRoleAttestationInput, crypto, keys,
    root_canister_sig::RootPayloadKind, verify,
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
        ic::{IcOps, ecdsa::EcdsaOps},
        storage::auth::AuthStateOps,
    },
};
use std::{cell::RefCell, collections::BTreeMap};

thread_local! {
    static PENDING_ROLE_ATTESTATIONS: RefCell<BTreeMap<PendingRoleAttestationKey, PreparedRootRoleAttestation>> =
        const { RefCell::new(BTreeMap::new()) };
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PendingRoleAttestationKey {
    payload_hash: [u8; 32],
    prepared_by: Vec<u8>,
}

impl PendingRoleAttestationKey {
    fn new(payload_hash: [u8; 32], prepared_by: Principal) -> Self {
        Self {
            payload_hash,
            prepared_by: prepared_by.as_slice().to_vec(),
        }
    }
}

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

    pub(crate) fn prepare_role_attestation(
        input: SignRoleAttestationInput,
    ) -> Result<PreparedRootRoleAttestation, InternalError> {
        let expires_at_ns = input
            .issued_at_ns
            .checked_add(input.ttl_ns)
            .ok_or_else(|| {
                AuthValidationError::Auth(
                    "role attestation ttl_ns overflows nanoseconds".to_string(),
                )
            })?;
        let payload = RoleAttestation {
            subject: input.subject,
            role: input.role,
            subnet_id: input.subnet_id,
            audience: input.audience,
            issued_at_ns: input.issued_at_ns,
            expires_at_ns,
            epoch: input.epoch,
        };
        let payload_hash = crypto::role_attestation_hash(&payload)?;
        let prepared_root_signature = Self::prepare_root_canister_signature(
            RootPayloadKind::RoleAttestation,
            input.operation_id,
            payload_hash,
            input.subject,
            input.issued_at_ns,
        )?;
        let prepared = PreparedRootRoleAttestation {
            payload,
            payload_hash,
            retrieval_expires_at_ns: prepared_root_signature.retrieval_expires_at_ns,
        };
        PENDING_ROLE_ATTESTATIONS.with_borrow_mut(|pending| {
            pending.insert(
                PendingRoleAttestationKey::new(payload_hash, input.subject),
                prepared.clone(),
            );
        });

        Ok(prepared)
    }

    pub(crate) fn get_role_attestation(
        caller: Principal,
        payload_hash: [u8; 32],
    ) -> Result<SignedRoleAttestation, InternalError> {
        let key = PendingRoleAttestationKey::new(payload_hash, caller);
        let prepared = PENDING_ROLE_ATTESTATIONS.with_borrow(|pending| pending.get(&key).cloned());
        let prepared = prepared.ok_or_else(|| {
            AuthValidationError::Auth(
                "role attestation was not prepared or has been pruned".to_string(),
            )
        })?;
        let root_proof = Self::get_root_canister_signature_proof(
            RootPayloadKind::RoleAttestation,
            payload_hash,
            caller,
            IcOps::canister_self(),
            IcOps::now_nanos(),
        )?;

        Ok(SignedRoleAttestation {
            payload: prepared.payload,
            root_proof,
        })
    }

    pub(crate) fn verify_role_attestation_cached(
        attestation: &SignedRoleAttestation,
        caller: Principal,
        self_pid: Principal,
        verifier_subnet: Option<Principal>,
        now_ns: u64,
        min_accepted_epoch: u64,
    ) -> Result<RoleAttestation, AuthOpsError> {
        let payload_hash = crypto::role_attestation_hash(&attestation.payload)
            .map_err(|err| AuthSignatureError::AttestationSignatureInvalid(err.to_string()))?;
        let verifier_cfg = Self::delegated_token_verifier_config()
            .map_err(|err| AuthValidationError::Auth(err.to_string()))?;
        Self::verify_root_canister_signature_proof(
            RootPayloadKind::RoleAttestation,
            payload_hash,
            &attestation.root_proof,
            verifier_cfg.root_canister_id,
            &verifier_cfg.ic_root_public_key_raw,
        )
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
