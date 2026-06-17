use super::{
    AuthOps, PrepareRootDelegationProofInput, PreparedRootDelegationProof,
    delegated::{
        active_proof::{
            InstallActiveDelegationProofError, InstallActiveDelegationProofInput,
            install_active_delegation_proof as build_active_delegation_proof,
        },
        cert_rules::DelegatedAuthTtlLimits,
        delegation_cert::{
            PrepareDelegationCertError, PrepareDelegationCertInput, finish_delegation_proof,
            prepare_delegation_cert,
        },
    },
    issuer_canister_sig::{IssuerPayloadKind, issuer_canister_sig_seed_hash},
    root_canister_sig::RootPayloadKind,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    domain::policy::auth::{
        AuthPolicyError, RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy,
        RootDelegationProofPreparePolicyInput, RootIssuerPolicy,
        validate_root_delegation_proof_prepare_policy,
    },
    dto::{
        auth::{
            ActiveDelegationProof, ActiveDelegationProofStatus,
            ActiveDelegationProofStatusResponse, DelegatedRoleGrant, DelegationAudience,
            DelegationProof, IssuerProofAlgorithm, IssuerProofBinding,
            RootDelegationProofBatchPrepareRequest,
        },
        error::Error,
    },
    ops::{auth::AuthValidationError, ic::IcOps, storage::auth::AuthStateOps},
};
use std::{cell::RefCell, collections::BTreeMap};

const DEFAULT_ROOT_PROVISIONING_REFRESH_AFTER_RATIO_BPS: u16 = 8_000;

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
        input: PrepareRootDelegationProofInput,
    ) -> Result<PreparedRootDelegationProof, InternalError> {
        let root_pid = IcOps::canister_self();
        let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 {
            seed_hash: issuer_canister_sig_seed_hash(IssuerPayloadKind::DelegatedTokenClaims),
        };

        let prepared = prepare_delegation_cert(PrepareDelegationCertInput {
            root_pid,
            issuer_pid: input.issuer_pid,
            issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
            issuer_proof_binding,
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
        .map_err(map_prepare_delegation_cert_error)?;
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
            super::delegated::delegation_cert::PreparedDelegationCert {
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
        let cfg = Self::auth_proof_verifier_config()?;
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

    pub(crate) fn active_delegation_proof_status(
        now_ns: u64,
    ) -> ActiveDelegationProofStatusResponse {
        active_delegation_proof_status_response(
            now_ns,
            AuthStateOps::active_delegation_proof_snapshot(),
        )
    }

    pub(crate) fn set_active_delegation_proof(proof: ActiveDelegationProof) {
        AuthStateOps::set_active_delegation_proof(proof);
    }

    pub(crate) fn preflight_delegation_proof_batch_prepare_request(
        request: &RootDelegationProofBatchPrepareRequest,
        max_cert_ttl_ns: u64,
        issued_at_ns: u64,
    ) -> Result<(), InternalError> {
        for entry in &request.entries {
            let audience = audience_policy(&entry.aud);
            let grants = grant_policies(&entry.grants);
            let policy = RootIssuerPolicy {
                issuer_pid: entry.issuer_pid,
                enabled: true,
                allowed_audiences: vec![audience.clone()],
                allowed_grants: grants.clone(),
                max_cert_ttl_ns,
                refresh_after_ratio_bps: DEFAULT_ROOT_PROVISIONING_REFRESH_AFTER_RATIO_BPS,
            };

            validate_root_delegation_proof_prepare_policy(
                Some(&policy),
                RootDelegationProofPreparePolicyInput {
                    issuer_pid: entry.issuer_pid,
                    audience: &audience,
                    grants: &grants,
                    cert_ttl_ns: entry.cert_ttl_ns,
                    issued_at_ns,
                },
            )
            .map_err(map_root_provisioning_policy_error)?;
        }

        Ok(())
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

fn map_prepare_delegation_cert_error(err: PrepareDelegationCertError) -> InternalError {
    AuthValidationError::Auth(err.to_string()).into()
}

fn map_install_active_delegation_proof_error(
    err: InstallActiveDelegationProofError,
) -> InternalError {
    AuthValidationError::Auth(err.to_string()).into()
}

fn map_root_provisioning_policy_error(err: AuthPolicyError) -> InternalError {
    InternalError::public(Error::forbidden(err.to_string()))
}

fn audience_policy(audience: &DelegationAudience) -> RootDelegationAudiencePolicy {
    match audience {
        DelegationAudience::Canister(canister) => RootDelegationAudiencePolicy::Canister(*canister),
        DelegationAudience::CanicSubnet(subnet) => {
            RootDelegationAudiencePolicy::CanicSubnet(*subnet)
        }
        DelegationAudience::Project(project) => {
            RootDelegationAudiencePolicy::Project(project.clone())
        }
    }
}

fn grant_policies(grants: &[DelegatedRoleGrant]) -> Vec<RootDelegatedRoleGrantPolicy> {
    grants
        .iter()
        .map(|grant| RootDelegatedRoleGrantPolicy {
            target: grant.target.clone(),
            scopes: grant.scopes.clone(),
        })
        .collect()
}

fn active_delegation_proof_status_response(
    now_ns: u64,
    proof: Option<ActiveDelegationProof>,
) -> ActiveDelegationProofStatusResponse {
    let Some(proof) = proof else {
        return ActiveDelegationProofStatusResponse {
            status: ActiveDelegationProofStatus::Missing,
            root_pid: None,
            issuer_pid: None,
            cert_hash: None,
            expires_at_ns: None,
            refresh_after_ns: None,
        };
    };

    let status = if now_ns >= proof.expires_at_ns {
        ActiveDelegationProofStatus::Expired
    } else if now_ns >= proof.refresh_after_ns {
        ActiveDelegationProofStatus::RefreshNeeded
    } else {
        ActiveDelegationProofStatus::Valid
    };

    ActiveDelegationProofStatusResponse {
        status,
        root_pid: Some(proof.proof.cert.root_pid),
        issuer_pid: Some(proof.proof.cert.issuer_pid),
        cert_hash: Some(proof.cert_hash),
        expires_at_ns: Some(proof.expires_at_ns),
        refresh_after_ns: Some(proof.refresh_after_ns),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::auth::{
            DelegatedRoleGrant, DelegationAudience, DelegationCert, IcCanisterSignatureProofV1,
            RootDelegationProofBatchPrepareEntry, RootDelegationProofBatchPrepareRequest,
            RootProof,
        },
        dto::error::ErrorCode,
        ids::CanisterRole,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn active_proof() -> ActiveDelegationProof {
        ActiveDelegationProof {
            proof: DelegationProof {
                cert: DelegationCert {
                    root_pid: p(1),
                    issuer_pid: p(2),
                    issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
                    issuer_proof_binding_hash: [3; 32],
                    issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 {
                        seed_hash: [4; 32],
                    },
                    issued_at_ns: 10,
                    not_before_ns: 20,
                    expires_at_ns: 100,
                    max_token_ttl_ns: 30,
                    aud: DelegationAudience::CanicSubnet(p(7)),
                    grants: vec![DelegatedRoleGrant {
                        target: CanisterRole::owned("project_instance".to_string()),
                        scopes: vec!["canic.issue".to_string()],
                    }],
                },
                root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                    signature_cbor: vec![8; 64],
                    public_key_der: vec![9; 32],
                }),
            },
            cert_hash: [10; 32],
            not_before_ns: 20,
            expires_at_ns: 100,
            refresh_after_ns: 80,
            installed_at_ns: 20,
            installed_by: p(11),
        }
    }

    fn batch_prepare_request(cert_ttl_ns: u64) -> RootDelegationProofBatchPrepareRequest {
        RootDelegationProofBatchPrepareRequest {
            metadata: None,
            entries: vec![RootDelegationProofBatchPrepareEntry {
                issuer_pid: p(2),
                aud: DelegationAudience::Project("test".to_string()),
                grants: vec![DelegatedRoleGrant {
                    target: CanisterRole::owned("project_instance".to_string()),
                    scopes: vec!["canic.issue".to_string()],
                }],
                cert_ttl_ns,
            }],
        }
    }

    #[test]
    fn active_delegation_proof_status_reports_missing() {
        let status = active_delegation_proof_status_response(50, None);

        assert_eq!(status.status, ActiveDelegationProofStatus::Missing);
        assert_eq!(status.root_pid, None);
        assert_eq!(status.cert_hash, None);
    }

    #[test]
    fn active_delegation_proof_status_reports_lifecycle_states() {
        let valid = active_delegation_proof_status_response(79, Some(active_proof()));
        assert_eq!(valid.status, ActiveDelegationProofStatus::Valid);
        assert_eq!(valid.root_pid, Some(p(1)));
        assert_eq!(valid.issuer_pid, Some(p(2)));
        assert_eq!(valid.cert_hash, Some([10; 32]));
        assert_eq!(valid.expires_at_ns, Some(100));
        assert_eq!(valid.refresh_after_ns, Some(80));

        let refresh = active_delegation_proof_status_response(80, Some(active_proof()));
        assert_eq!(refresh.status, ActiveDelegationProofStatus::RefreshNeeded);

        let expired = active_delegation_proof_status_response(100, Some(active_proof()));
        assert_eq!(expired.status, ActiveDelegationProofStatus::Expired);
    }

    #[test]
    fn batch_prepare_preflight_accepts_request_shape() {
        AuthOps::preflight_delegation_proof_batch_prepare_request(
            &batch_prepare_request(60_000_000_000),
            120_000_000_000,
            10,
        )
        .expect("valid batch prepare shape");
    }

    #[test]
    fn batch_prepare_preflight_rejects_ttl_above_max() {
        let err = AuthOps::preflight_delegation_proof_batch_prepare_request(
            &batch_prepare_request(121_000_000_000),
            120_000_000_000,
            10,
        )
        .expect_err("ttl above max must fail preflight");
        let public = err.public_error().expect("public policy error");

        assert_eq!(public.code, ErrorCode::Forbidden);
    }
}
