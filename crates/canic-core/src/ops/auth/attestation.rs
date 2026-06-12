use super::{
    AuthOps, PreparedRootRoleAttestation, SignRoleAttestationInput, crypto,
    root_canister_sig::RootPayloadKind, verify,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{RoleAttestation, SignedRoleAttestation},
    ops::{
        auth::{AuthOpsError, AuthSignatureError, AuthValidationError},
        ic::IcOps,
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
        let prepared_root_proof = Self::prepare_root_canister_signature(
            RootPayloadKind::RoleAttestation,
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
            .map_err(|err| AuthSignatureError::AttestationProofInvalid(err.to_string()))?;
        let verifier_cfg = Self::delegated_token_verifier_config()
            .map_err(|err| AuthValidationError::Auth(err.to_string()))?;
        Self::verify_root_canister_signature_proof(
            RootPayloadKind::RoleAttestation,
            payload_hash,
            &attestation.root_proof,
            verifier_cfg.root_canister_id,
            &verifier_cfg.ic_root_public_key_raw,
        )
        .map_err(|err| AuthSignatureError::AttestationProofInvalid(err.to_string()))?;

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        dto::auth::{IcCanisterSignatureProofV1, RootProof},
        test::config::ConfigTestBuilder,
    };

    fn p(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }

    #[test]
    fn role_attestation_verifier_uses_same_mainnet_root_key_requirement() {
        let mut cfg = ConfigTestBuilder::new().build();
        cfg.auth.delegated_tokens.network = "mainnet".to_string();
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
            root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                signature_cbor: vec![1, 2, 3],
                public_key_der: vec![4, 5, 6],
            }),
        };

        let err = AuthOps::verify_role_attestation_cached(&attestation, p(2), p(3), None, 15, 0)
            .expect_err("missing mainnet root key must fail before proof acceptance");

        assert!(
            err.to_string()
                .contains("ic_root_public_key_raw_hex is required"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn role_attestation_verifier_requires_explicit_local_root_key() {
        let mut cfg = ConfigTestBuilder::new().build();
        cfg.auth.delegated_tokens.network = "local".to_string();
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
            root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                signature_cbor: vec![1, 2, 3],
                public_key_der: vec![4, 5, 6],
            }),
        };

        let err = AuthOps::verify_role_attestation_cached(&attestation, p(2), p(3), None, 15, 0)
            .expect_err("local verifier must fail before proof acceptance without root key");

        assert!(
            err.to_string()
                .contains("ic_root_public_key_raw_hex is required"),
            "unexpected error: {err}"
        );
    }
}
