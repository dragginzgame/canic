use super::{
    audience::{AudienceError, validate_audience_shape, validate_role_grants},
    canonical::{CanonicalAuthError, cert_hash, issuer_proof_binding_hash},
    cert_rules::{CertRuleError, DelegatedAuthTtlLimits},
};
use crate::{
    cdk::types::Principal,
    dto::auth::{
        DelegatedRoleGrant, DelegationAudience, DelegationCert, DelegationProof,
        IssuerProofAlgorithm, IssuerProofBinding, RootProof,
    },
};
use thiserror::Error;

pub struct IssueDelegationProofInput {
    pub root_pid: Principal,
    pub issuer_pid: Principal,
    pub issuer_proof_alg: IssuerProofAlgorithm,
    pub issuer_proof_binding: IssuerProofBinding,
    pub issued_at_ns: u64,
    pub cert_ttl_ns: u64,
    pub max_token_ttl_ns: u64,
    pub audience: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub ttl_limits: DelegatedAuthTtlLimits,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuedDelegationProof {
    pub proof: DelegationProof,
    pub cert_hash: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedDelegationCert {
    pub cert: DelegationCert,
    pub cert_hash: [u8; 32],
}

#[derive(Debug, Eq, Error, PartialEq)]
pub enum IssueDelegationProofError {
    #[error("delegated auth cert ttl must be greater than zero")]
    CertTtlZero,
    #[error("delegated auth cert expires_at overflow")]
    CertExpiresAtOverflow,
    #[error(transparent)]
    Audience(#[from] AudienceError),
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthError),
    #[error(transparent)]
    CertRules(#[from] CertRuleError),
}

/// Build one self-validating delegation proof from an already-created root proof.
#[cfg(test)]
pub fn issue_delegation_proof(
    input: IssueDelegationProofInput,
    root_proof: RootProof,
) -> Result<IssuedDelegationProof, IssueDelegationProofError> {
    let prepared = prepare_delegation_cert(input)?;
    Ok(finish_delegation_proof(prepared, root_proof))
}

/// Prepare one canonical delegation certificate before root proof creation.
pub fn prepare_delegation_cert(
    input: IssueDelegationProofInput,
) -> Result<PreparedDelegationCert, IssueDelegationProofError> {
    if input.cert_ttl_ns == 0 {
        return Err(IssueDelegationProofError::CertTtlZero);
    }

    validate_audience_shape(&input.audience)?;
    validate_role_grants(&input.grants)?;

    let expires_at = input
        .issued_at_ns
        .checked_add(input.cert_ttl_ns)
        .ok_or(IssueDelegationProofError::CertExpiresAtOverflow)?;
    let issuer_proof_binding_hash = issuer_proof_binding_hash(
        input.issuer_pid,
        input.issuer_proof_alg,
        input.issuer_proof_binding,
    );

    let cert = DelegationCert {
        root_pid: input.root_pid,
        issuer_pid: input.issuer_pid,
        issuer_proof_alg: input.issuer_proof_alg,
        issuer_proof_binding_hash,
        issuer_proof_binding: input.issuer_proof_binding,
        issued_at_ns: input.issued_at_ns,
        not_before_ns: input.issued_at_ns,
        expires_at_ns: expires_at,
        max_token_ttl_ns: input.max_token_ttl_ns,
        aud: input.audience,
        grants: input.grants,
    };

    validate_cert_issuance_rules_for_built_cert(&cert, input.ttl_limits)?;

    let cert_hash = cert_hash(&cert)?;

    Ok(PreparedDelegationCert { cert, cert_hash })
}

/// Combine a prepared certificate with its root proof.
pub fn finish_delegation_proof(
    prepared: PreparedDelegationCert,
    root_proof: RootProof,
) -> IssuedDelegationProof {
    IssuedDelegationProof {
        proof: DelegationProof {
            cert: prepared.cert,
            root_proof,
        },
        cert_hash: prepared.cert_hash,
    }
}

fn validate_cert_issuance_rules_for_built_cert(
    cert: &DelegationCert,
    ttl_limits: DelegatedAuthTtlLimits,
) -> Result<(), CertRuleError> {
    super::cert_rules::validate_cert_issuance_rules(cert, ttl_limits, cert.root_pid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids::CanisterRole,
        ops::auth::issuer_canister_sig::{IssuerPayloadKind, issuer_canister_sig_seed_hash},
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn ttl_limits() -> DelegatedAuthTtlLimits {
        DelegatedAuthTtlLimits {
            max_cert_ttl_ns: 600,
            max_token_ttl_ns: 120,
        }
    }

    fn input() -> IssueDelegationProofInput {
        IssueDelegationProofInput {
            root_pid: p(1),
            issuer_pid: p(2),
            issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
            issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 {
                seed_hash: issuer_canister_sig_seed_hash(IssuerPayloadKind::DelegatedTokenClaims),
            },
            issued_at_ns: 100,
            cert_ttl_ns: 400,
            max_token_ttl_ns: 120,
            audience: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("project_instance", &["read", "write"])],
            ttl_limits: ttl_limits(),
        }
    }

    fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
            target: CanisterRole::owned(role.to_string()),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        }
    }

    fn root_proof(byte: u8) -> RootProof {
        RootProof::IcCanisterSignatureV1(crate::dto::auth::IcCanisterSignatureProofV1 {
            signature_cbor: vec![byte; 8],
            public_key_der: vec![byte; 4],
        })
    }

    #[test]
    fn issue_delegation_proof_embeds_exact_root_proof() {
        let expected_root_proof = root_proof(9);
        let issued = issue_delegation_proof(input(), expected_root_proof.clone()).unwrap();

        assert_eq!(issued.proof.cert.root_pid, p(1));
        assert_eq!(issued.proof.cert.issuer_pid, p(2));
        assert_eq!(issued.proof.cert.issued_at_ns, 100);
        assert_eq!(issued.proof.cert.expires_at_ns, 500);
        assert_eq!(
            issued.proof.cert.issuer_proof_binding_hash,
            issuer_proof_binding_hash(
                p(2),
                IssuerProofAlgorithm::IcCanisterSignatureV1,
                IssuerProofBinding::IcCanisterSignatureV1 {
                    seed_hash: issuer_canister_sig_seed_hash(
                        IssuerPayloadKind::DelegatedTokenClaims
                    ),
                },
            )
        );
        assert_eq!(issued.cert_hash, cert_hash(&issued.proof.cert).unwrap());
        assert_eq!(issued.proof.root_proof, expected_root_proof);
    }

    #[test]
    fn issue_delegation_proof_rejects_empty_grants() {
        let mut input = input();
        input.grants = vec![];

        assert_eq!(
            issue_delegation_proof(input, root_proof(1)),
            Err(IssueDelegationProofError::Audience(
                AudienceError::GrantsEmpty
            ))
        );
    }

    #[test]
    fn issue_delegation_proof_rejects_cert_ttl_above_limits() {
        let mut input = input();
        input.cert_ttl_ns = 601;

        assert_eq!(
            issue_delegation_proof(input, root_proof(1)),
            Err(IssueDelegationProofError::CertRules(
                CertRuleError::CertTtlExceeded {
                    ttl_ns: 601,
                    max_ttl_ns: 600,
                }
            ))
        );
    }

    #[test]
    fn issue_delegation_proof_rejects_invalid_grant_role() {
        let mut input = input();
        input.grants = vec![DelegatedRoleGrant {
            target: CanisterRole::owned("ProjectInstance".to_string()),
            scopes: vec!["read".to_string()],
        }];

        assert_eq!(
            issue_delegation_proof(input, root_proof(1)),
            Err(IssueDelegationProofError::Audience(
                AudienceError::Canonical(super::CanonicalAuthError::InvalidRole {
                    role: "ProjectInstance".to_string(),
                })
            ))
        );
    }
}
