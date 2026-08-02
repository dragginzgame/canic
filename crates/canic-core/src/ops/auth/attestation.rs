//! Module: ops::auth::attestation
//!
//! Responsibility: prepare, retrieve, and verify root role attestation proofs.
//! Does not own: endpoint authorization, role policy, or public DTO schemas.
//! Boundary: auth ops facade for role-attestation workflows and root proof helpers.

use super::{
    AuthOps, PrepareRootRoleAttestationInput, PreparedRootRoleAttestation, crypto, verify,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{RoleAttestation, RoleAttestationRootProof, SignedRoleAttestation},
    ops::{
        auth::{AuthOpsError, AuthSignatureError, AuthValidationError},
        ic::IcOps,
    },
};
use std::{cell::RefCell, collections::BTreeMap};

const ROLE_ATTESTATION_SIGNATURE_CBOR_MAX_BYTES: usize = 256 * 1024;
const ROLE_ATTESTATION_PUBLIC_KEY_DER_MAX_BYTES: usize = 4 * 1024;

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
    pub(crate) fn prepare_role_attestation(
        input: PrepareRootRoleAttestationInput,
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
        let prepared_root_proof = Self::prepare_root_canister_signature(
            input.operation_id,
            payload_hash,
            input.subject,
            input.issued_at_ns,
        )?;
        let prepared = PreparedRootRoleAttestation {
            payload,
            payload_hash,
            retrieval_expires_at_ns: prepared_root_proof.retrieval_expires_at_ns,
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
        verify_role_attestation_proof(attestation)?;

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

    pub(crate) fn verify_local_subnet_role_attestation_cached(
        attestation: &SignedRoleAttestation,
        caller: Principal,
        self_pid: Principal,
        verifier_subnet: Principal,
        now_ns: u64,
        min_accepted_epoch: u64,
    ) -> Result<RoleAttestation, AuthOpsError> {
        verify_role_attestation_proof(attestation)?;

        verify::verify_local_subnet_role_attestation_claims(
            &attestation.payload,
            caller,
            self_pid,
            verifier_subnet,
            now_ns,
            min_accepted_epoch,
        )?;

        Ok(attestation.payload.clone())
    }
}

fn verify_role_attestation_proof(attestation: &SignedRoleAttestation) -> Result<(), AuthOpsError> {
    validate_role_attestation_proof_bounds(&attestation.root_proof)?;
    let payload_hash = crypto::role_attestation_hash(&attestation.payload)
        .map_err(|err| AuthSignatureError::AttestationProofInvalid(err.to_string()))?;
    let verifier_cfg = AuthOps::auth_proof_verifier_config()
        .map_err(|err| AuthValidationError::Auth(err.to_string()))?;
    AuthOps::verify_root_canister_signature_proof(
        payload_hash,
        &attestation.root_proof,
        verifier_cfg.root_canister_id,
        &verifier_cfg.ic_root_public_key_raw,
    )
    .map_err(|err| AuthSignatureError::AttestationProofInvalid(err.to_string()))?;
    Ok(())
}

fn validate_role_attestation_proof_bounds(
    proof: &RoleAttestationRootProof,
) -> Result<(), AuthValidationError> {
    let RoleAttestationRootProof::IcCanisterSignatureV1(proof) = proof;
    require_proof_field_bound(
        "signature_cbor",
        proof.signature_cbor.len(),
        ROLE_ATTESTATION_SIGNATURE_CBOR_MAX_BYTES,
    )?;
    require_proof_field_bound(
        "public_key_der",
        proof.public_key_der.len(),
        ROLE_ATTESTATION_PUBLIC_KEY_DER_MAX_BYTES,
    )
}

const fn require_proof_field_bound(
    field: &'static str,
    actual_bytes: usize,
    max_bytes: usize,
) -> Result<(), AuthValidationError> {
    if actual_bytes > max_bytes {
        return Err(AuthValidationError::AttestationProofFieldTooLarge {
            field,
            actual_bytes,
            max_bytes,
        });
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        dto::auth::{IcCanisterSignatureProofV1, RoleAttestationRootProof},
        ids::BuildNetwork,
        test::config::ConfigTestBuilder,
    };

    fn p(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }

    #[test]
    fn role_attestation_verifier_uses_same_ic_root_key_requirement() {
        let mut cfg = ConfigTestBuilder::new().build();
        cfg.auth.delegated_tokens.build_network = BuildNetwork::Ic;
        cfg.auth.delegated_tokens.root_canister_id = Some(p(1).to_string());
        cfg.auth.delegated_tokens.ic_root_public_key_raw_hex = None;
        Config::reset_for_tests();
        Config::init_from_model_for_tests(cfg).expect("test config should install");

        let attestation = SignedRoleAttestation {
            payload: RoleAttestation {
                subject: p(2),
                role: crate::ids::CanisterRole::new("project_hub"),
                subnet_id: None,
                audience: p(3),
                issued_at_ns: 10,
                expires_at_ns: 20,
                epoch: 0,
            },
            root_proof: RoleAttestationRootProof::IcCanisterSignatureV1(
                IcCanisterSignatureProofV1 {
                    signature_cbor: vec![1, 2, 3],
                    public_key_der: vec![4, 5, 6],
                },
            ),
        };

        AuthOps::verify_role_attestation_cached(&attestation, p(2), p(3), None, 15, 0)
            .expect_err("missing IC root key must fail before proof acceptance");
    }

    #[test]
    fn role_attestation_verifier_requires_explicit_local_root_key() {
        let mut cfg = ConfigTestBuilder::new().build();
        cfg.auth.delegated_tokens.build_network = BuildNetwork::Local;
        cfg.auth.delegated_tokens.root_canister_id = Some(p(1).to_string());
        cfg.auth.delegated_tokens.ic_root_public_key_raw_hex = None;
        Config::reset_for_tests();
        Config::init_from_model_for_tests(cfg).expect("test config should install");

        let attestation = SignedRoleAttestation {
            payload: RoleAttestation {
                subject: p(2),
                role: crate::ids::CanisterRole::new("project_hub"),
                subnet_id: None,
                audience: p(3),
                issued_at_ns: 10,
                expires_at_ns: 20,
                epoch: 0,
            },
            root_proof: RoleAttestationRootProof::IcCanisterSignatureV1(
                IcCanisterSignatureProofV1 {
                    signature_cbor: vec![1, 2, 3],
                    public_key_der: vec![4, 5, 6],
                },
            ),
        };

        AuthOps::verify_role_attestation_cached(&attestation, p(2), p(3), None, 15, 0)
            .expect_err("local verifier must fail before proof acceptance without root key");
    }

    #[test]
    fn role_attestation_proof_bounds_reject_oversized_fields_before_crypto() {
        let oversized_signature =
            RoleAttestationRootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                signature_cbor: vec![0; ROLE_ATTESTATION_SIGNATURE_CBOR_MAX_BYTES + 1],
                public_key_der: Vec::new(),
            });

        let err = validate_role_attestation_proof_bounds(&oversized_signature)
            .expect_err("oversized signature proof must fail before verification");
        std::assert_matches!(
            err,
            AuthValidationError::AttestationProofFieldTooLarge {
                field: "signature_cbor",
                ..
            }
        );

        let oversized_key =
            RoleAttestationRootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                signature_cbor: Vec::new(),
                public_key_der: vec![0; ROLE_ATTESTATION_PUBLIC_KEY_DER_MAX_BYTES + 1],
            });
        let err = validate_role_attestation_proof_bounds(&oversized_key)
            .expect_err("oversized public key must fail before verification");
        std::assert_matches!(
            err,
            AuthValidationError::AttestationProofFieldTooLarge {
                field: "public_key_der",
                ..
            }
        );
    }
}
