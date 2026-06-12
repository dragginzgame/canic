use super::{
    AuthOps, PreparedRootDelegationProof, SignDelegationProofInput,
    delegated::{
        active_proof::{
            InstallActiveDelegationProofError, InstallActiveDelegationProofInput,
            install_active_delegation_proof as build_active_delegation_proof,
        },
        cert_rules::DelegatedAuthTtlLimits,
        issue::{
            IssueDelegationProofError, IssueDelegationProofInput, finish_delegation_proof,
            prepare_delegation_cert,
        },
    },
    issuer_canister_sig::{IssuerPayloadKind, issuer_sig_seed_hash},
    root_canister_sig::RootPayloadKind,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{ActiveDelegationProof, DelegationProof, IssuerProofAlgorithm, IssuerProofBinding},
    ops::{auth::AuthValidationError, ic::IcOps, storage::auth::AuthStateOps},
};
use std::{cell::RefCell, collections::BTreeMap};

thread_local! {
    static PENDING_DELEGATION_PROOFS: RefCell<BTreeMap<PendingDelegationProofKey, PreparedRootDelegationProof>> =
        const { RefCell::new(BTreeMap::new()) };
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PendingDelegationProofKey {
    cert_hash: [u8; 32],
    prepared_by: Vec<u8>,
}

impl PendingDelegationProofKey {
    fn new(cert_hash: [u8; 32], prepared_by: Principal) -> Self {
        Self {
            cert_hash,
            prepared_by: prepared_by.as_slice().to_vec(),
        }
    }
}

impl AuthOps {
    /// Prepare a canonical delegation proof certificate and certify its canister-signature path.
    pub(crate) fn prepare_delegation_proof(
        input: SignDelegationProofInput,
    ) -> Result<PreparedRootDelegationProof, InternalError> {
        let root_pid = IcOps::canister_self();
        let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 {
            seed_hash: issuer_sig_seed_hash(IssuerPayloadKind::DelegatedTokenClaims),
        };

        let prepared = prepare_delegation_cert(IssueDelegationProofInput {
            root_pid,
            issuer_pid: input.issuer_pid,
            issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
            issuer_proof_binding,
            issuer_signer_generation: None,
            issued_at_ns: input.issued_at_ns,
            cert_ttl_ns: input.cert_ttl_ns,
            max_token_ttl_ns: input.max_token_ttl_ns,
            audience: input.audience,
            grants: input.grants,
            ttl_limits: DelegatedAuthTtlLimits {
                max_cert_ttl_ns: input.max_cert_ttl_ns,
                max_token_ttl_ns: input.max_token_ttl_ns,
            },
        })
        .map_err(map_issue_delegation_proof_error)?;
        let prepared_root_proof = Self::prepare_root_canister_signature(
            RootPayloadKind::DelegationCert,
            input.operation_id,
            prepared.cert_hash,
            input.issuer_pid,
            input.issued_at_ns,
        )?;
        let prepared = PreparedRootDelegationProof {
            cert: prepared.cert,
            cert_hash: prepared.cert_hash,
            retrieval_expires_at_ns: prepared_root_proof.retrieval_expires_at_ns,
        };
        cache_prepared_delegation_proof(input.issuer_pid, prepared.clone());

        Ok(prepared)
    }

    /// Finish an already-prepared root delegation proof from query-only certificate material.
    pub(crate) fn get_delegation_proof(
        caller: Principal,
        cert_hash: [u8; 32],
    ) -> Result<DelegationProof, InternalError> {
        let prepared = PENDING_DELEGATION_PROOFS
            .with(|pending| {
                pending
                    .borrow()
                    .get(&PendingDelegationProofKey::new(cert_hash, caller))
                    .cloned()
            })
            .ok_or_else(|| {
                AuthValidationError::Auth(
                    "delegation proof was not prepared or has expired".to_string(),
                )
            })?;
        let root_proof = Self::get_root_canister_signature_proof(
            RootPayloadKind::DelegationCert,
            prepared.cert_hash,
            caller,
            prepared.cert.root_pid,
            IcOps::now_nanos(),
        )?;
        Ok(finish_delegation_proof(
            super::delegated::issue::PreparedDelegationCert {
                cert: prepared.cert,
                cert_hash: prepared.cert_hash,
            },
            root_proof,
        )
        .proof)
    }

    pub(crate) fn install_active_delegation_proof(
        proof: DelegationProof,
        installed_by: Principal,
    ) -> Result<ActiveDelegationProof, InternalError> {
        let cfg = Self::delegated_token_verifier_config()?;
        let now_ns = IcOps::now_nanos();
        let active_proof = build_active_delegation_proof(
            InstallActiveDelegationProofInput {
                proof,
                installed_by,
                this_canister: IcOps::canister_self(),
                now_ns,
            },
            |cert_hash, root_proof, root_pid| {
                if root_pid != cfg.root_canister_id {
                    return Err(AuthValidationError::InvalidRootAuthority {
                        expected: cfg.root_canister_id,
                        found: root_pid,
                    }
                    .to_string());
                }
                Self::verify_root_canister_signature_proof(
                    RootPayloadKind::DelegationCert,
                    cert_hash,
                    root_proof,
                    cfg.root_canister_id,
                    &cfg.ic_root_public_key_raw,
                )
                .map_err(|err| err.to_string())
            },
        )
        .map_err(map_install_active_delegation_proof_error)?;

        Self::set_active_delegation_proof(active_proof.clone());
        Ok(active_proof)
    }

    #[must_use]
    pub(crate) fn active_delegation_proof(now_ns: u64) -> Option<ActiveDelegationProof> {
        AuthStateOps::active_delegation_proof(now_ns)
    }

    pub(crate) fn set_active_delegation_proof(proof: ActiveDelegationProof) {
        AuthStateOps::set_active_delegation_proof(proof);
    }

    #[expect(
        dead_code,
        reason = "active delegation proof install endpoint lands with issuer prepare/get flow"
    )]
    pub(crate) fn clear_active_delegation_proof() {
        AuthStateOps::clear_active_delegation_proof();
    }
}

fn cache_prepared_delegation_proof(caller: Principal, prepared: PreparedRootDelegationProof) {
    PENDING_DELEGATION_PROOFS.with(|pending| {
        pending.borrow_mut().insert(
            PendingDelegationProofKey::new(prepared.cert_hash, caller),
            prepared,
        );
    });
}

fn map_issue_delegation_proof_error(err: IssueDelegationProofError) -> InternalError {
    AuthValidationError::Auth(err.to_string()).into()
}

fn map_install_active_delegation_proof_error(
    err: InstallActiveDelegationProofError,
) -> InternalError {
    AuthValidationError::Auth(err.to_string()).into()
}
