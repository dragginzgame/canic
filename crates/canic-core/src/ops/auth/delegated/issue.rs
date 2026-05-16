use super::{
    audience::{AudienceError, expected_role_hash_for_cert_audience},
    canonical::{CanonicalAuthError, cert_hash, public_key_hash},
    cert_rules::{CertRuleError, DELEGATED_AUTH_VERSION, DelegatedAuthTtlLimits},
};
use crate::{
    cdk::types::Principal,
    dto::auth::{
        DelegationAudience, DelegationCert, DelegationProof, ShardKeyBinding, SignatureAlgorithm,
    },
};
use thiserror::Error;

pub struct IssueDelegationProofInput {
    pub root_pid: Principal,
    pub root_key_id: String,
    pub root_public_key: Vec<u8>,
    pub shard_pid: Principal,
    pub shard_key_id: String,
    pub shard_public_key_sec1: Vec<u8>,
    pub shard_key_binding: ShardKeyBinding,
    pub issued_at: u64,
    pub cert_ttl_secs: u64,
    pub max_token_ttl_secs: u64,
    pub scopes: Vec<String>,
    pub audience: DelegationAudience,
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
    #[error("delegated auth cert scopes must not be empty")]
    ScopesEmpty,
    #[error("delegated auth cert scope is empty")]
    ScopeEmpty,
    #[cfg(test)]
    #[error("delegated auth root signature failed: {0}")]
    SignFailed(String),
    #[error(transparent)]
    Audience(#[from] AudienceError),
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthError),
    #[error(transparent)]
    CertRules(#[from] CertRuleError),
}

/// Build and sign one self-validating delegation proof.
#[cfg(test)]
pub fn issue_delegation_proof<F>(
    input: IssueDelegationProofInput,
    sign_cert_hash: F,
) -> Result<IssuedDelegationProof, IssueDelegationProofError>
where
    F: FnOnce([u8; 32]) -> Result<Vec<u8>, String>,
{
    let prepared = prepare_delegation_cert(input)?;
    let root_sig =
        sign_cert_hash(prepared.cert_hash).map_err(IssueDelegationProofError::SignFailed)?;
    Ok(finish_delegation_proof(prepared, root_sig))
}

/// Prepare one canonical delegation certificate before root signing.
pub fn prepare_delegation_cert(
    input: IssueDelegationProofInput,
) -> Result<PreparedDelegationCert, IssueDelegationProofError> {
    if input.cert_ttl_secs == 0 {
        return Err(IssueDelegationProofError::CertTtlZero);
    }

    validate_scopes(&input.scopes)?;

    let expires_at = input
        .issued_at
        .checked_add(input.cert_ttl_secs)
        .ok_or(IssueDelegationProofError::CertExpiresAtOverflow)?;
    let root_key_hash = public_key_hash(&input.root_public_key);
    let shard_key_hash = public_key_hash(&input.shard_public_key_sec1);
    let verifier_role_hash = expected_role_hash_for_cert_audience(&input.audience)?;

    let cert = DelegationCert {
        version: DELEGATED_AUTH_VERSION,
        root_pid: input.root_pid,
        root_key_id: input.root_key_id,
        root_key_hash,
        alg: SignatureAlgorithm::EcdsaP256Sha256,
        shard_pid: input.shard_pid,
        shard_key_id: input.shard_key_id,
        shard_public_key_sec1: input.shard_public_key_sec1,
        shard_key_hash,
        shard_key_binding: input.shard_key_binding,
        issued_at: input.issued_at,
        expires_at,
        max_token_ttl_secs: input.max_token_ttl_secs,
        scopes: input.scopes,
        aud: input.audience,
        verifier_role_hash,
    };

    validate_cert_issuance_rules_for_built_cert(&cert, input.ttl_limits)?;

    let cert_hash = cert_hash(&cert)?;

    Ok(PreparedDelegationCert { cert, cert_hash })
}

/// Combine a prepared certificate with its root signature.
pub fn finish_delegation_proof(
    prepared: PreparedDelegationCert,
    root_sig: Vec<u8>,
) -> IssuedDelegationProof {
    IssuedDelegationProof {
        proof: DelegationProof {
            cert: prepared.cert,
            root_sig,
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

fn validate_scopes(scopes: &[String]) -> Result<(), IssueDelegationProofError> {
    if scopes.is_empty() {
        return Err(IssueDelegationProofError::ScopesEmpty);
    }
    if scopes.iter().any(String::is_empty) {
        return Err(IssueDelegationProofError::ScopeEmpty);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::CanisterRole;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn ttl_limits() -> DelegatedAuthTtlLimits {
        DelegatedAuthTtlLimits {
            max_cert_ttl_secs: 600,
            max_token_ttl_secs: 120,
        }
    }

    fn input() -> IssueDelegationProofInput {
        IssueDelegationProofInput {
            root_pid: p(1),
            root_key_id: "root-key".to_string(),
            root_public_key: vec![10, 11, 12],
            shard_pid: p(2),
            shard_key_id: "shard-key".to_string(),
            shard_public_key_sec1: vec![20, 21, 22],
            shard_key_binding: ShardKeyBinding::IcThresholdEcdsa {
                key_name_hash: [3; 32],
                derivation_path_hash: [4; 32],
            },
            issued_at: 100,
            cert_ttl_secs: 400,
            max_token_ttl_secs: 120,
            scopes: vec!["read".to_string(), "write".to_string()],
            audience: DelegationAudience::Roles(vec![CanisterRole::new("project_instance")]),
            ttl_limits: ttl_limits(),
        }
    }

    #[test]
    fn issue_delegation_proof_signs_exact_cert_hash() {
        let mut observed_hash = None;

        let issued = issue_delegation_proof(input(), |hash| {
            observed_hash = Some(hash);
            Ok(hash.to_vec())
        })
        .unwrap();

        assert_eq!(issued.proof.cert.version, DELEGATED_AUTH_VERSION);
        assert_eq!(issued.proof.cert.root_pid, p(1));
        assert_eq!(issued.proof.cert.issued_at, 100);
        assert_eq!(issued.proof.cert.expires_at, 500);
        assert_eq!(
            issued.proof.cert.root_key_hash,
            public_key_hash(&[10, 11, 12])
        );
        assert_eq!(
            issued.proof.cert.shard_key_hash,
            public_key_hash(&[20, 21, 22])
        );
        assert_eq!(
            issued.proof.cert.verifier_role_hash,
            expected_role_hash_for_cert_audience(&issued.proof.cert.aud).unwrap()
        );
        assert_eq!(issued.cert_hash, cert_hash(&issued.proof.cert).unwrap());
        assert_eq!(observed_hash, Some(issued.cert_hash));
        assert_eq!(issued.proof.root_sig, issued.cert_hash.to_vec());
    }

    #[test]
    fn issue_delegation_proof_rejects_empty_scopes() {
        let mut input = input();
        input.scopes = vec![];

        assert_eq!(
            issue_delegation_proof(input, |hash| Ok(hash.to_vec())),
            Err(IssueDelegationProofError::ScopesEmpty)
        );
    }

    #[test]
    fn issue_delegation_proof_rejects_cert_ttl_above_limits() {
        let mut input = input();
        input.cert_ttl_secs = 601;

        assert_eq!(
            issue_delegation_proof(input, |hash| Ok(hash.to_vec())),
            Err(IssueDelegationProofError::CertRules(
                CertRuleError::CertTtlExceeded {
                    ttl_secs: 601,
                    max_ttl_secs: 600,
                }
            ))
        );
    }

    #[test]
    fn issue_delegation_proof_rejects_multi_role_cert_audience() {
        let mut input = input();
        input.audience = DelegationAudience::Roles(vec![
            CanisterRole::new("project_instance"),
            CanisterRole::new("project_hub"),
        ]);

        assert_eq!(
            issue_delegation_proof(input, |hash| Ok(hash.to_vec())),
            Err(IssueDelegationProofError::Audience(
                AudienceError::RoleAudienceMustBeSingular
            ))
        );
    }

    #[test]
    fn issue_delegation_proof_rejects_signing_failure() {
        assert_eq!(
            issue_delegation_proof(input(), |_| Err("sign failed".to_string())),
            Err(IssueDelegationProofError::SignFailed(
                "sign failed".to_string()
            ))
        );
    }
}
