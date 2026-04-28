use super::{
    audience::{AudienceV2Error, expected_role_hash_for_cert_audience},
    canonical::{CanonicalAuthV2Error, cert_hash, public_key_hash},
    policy::{CertPolicyV2Error, DELEGATED_AUTH_V2_VERSION, DelegatedAuthTtlPolicyV2},
};
use crate::{
    cdk::types::Principal,
    dto::auth::{
        DelegationAudienceV2, DelegationCertV2, DelegationProofV2, RootKeyCertificateV2,
        ShardKeyBindingV2, SignatureAlgorithmV2,
    },
};
use thiserror::Error;

pub struct IssueDelegationProofV2Input {
    pub root_pid: Principal,
    pub root_key_id: String,
    pub root_public_key_sec1: Vec<u8>,
    pub root_key_cert: Option<RootKeyCertificateV2>,
    pub shard_pid: Principal,
    pub shard_key_id: String,
    pub shard_public_key_sec1: Vec<u8>,
    pub shard_key_binding: ShardKeyBindingV2,
    pub issued_at: u64,
    pub cert_ttl_secs: u64,
    pub max_token_ttl_secs: u64,
    pub scopes: Vec<String>,
    pub audience: DelegationAudienceV2,
    pub ttl_policy: DelegatedAuthTtlPolicyV2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuedDelegationProofV2 {
    pub proof: DelegationProofV2,
    pub cert_hash: [u8; 32],
}

#[derive(Debug, Eq, Error, PartialEq)]
pub enum IssueDelegationProofV2Error {
    #[error("delegated auth v2 cert ttl must be greater than zero")]
    CertTtlZero,
    #[error("delegated auth v2 cert expires_at overflow")]
    CertExpiresAtOverflow,
    #[error("delegated auth v2 cert scopes must not be empty")]
    ScopesEmpty,
    #[error("delegated auth v2 cert scope is empty")]
    ScopeEmpty,
    #[error("delegated auth v2 root signature failed: {0}")]
    SignFailed(String),
    #[error(transparent)]
    Audience(#[from] AudienceV2Error),
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthV2Error),
    #[error(transparent)]
    Policy(#[from] CertPolicyV2Error),
}

/// Build and sign one self-validating V2 delegation proof.
pub fn issue_delegation_proof_v2<F>(
    input: IssueDelegationProofV2Input,
    sign_cert_hash: F,
) -> Result<IssuedDelegationProofV2, IssueDelegationProofV2Error>
where
    F: FnOnce([u8; 32]) -> Result<Vec<u8>, String>,
{
    if input.cert_ttl_secs == 0 {
        return Err(IssueDelegationProofV2Error::CertTtlZero);
    }

    validate_scopes(&input.scopes)?;

    let expires_at = input
        .issued_at
        .checked_add(input.cert_ttl_secs)
        .ok_or(IssueDelegationProofV2Error::CertExpiresAtOverflow)?;
    let root_key_hash = public_key_hash(&input.root_public_key_sec1);
    let shard_key_hash = public_key_hash(&input.shard_public_key_sec1);
    let verifier_role_hash = expected_role_hash_for_cert_audience(&input.audience)?;

    let cert = DelegationCertV2 {
        version: DELEGATED_AUTH_V2_VERSION,
        root_pid: input.root_pid,
        root_key_id: input.root_key_id,
        root_key_hash,
        alg: SignatureAlgorithmV2::EcdsaP256Sha256,
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

    validate_cert_issuance_policy_for_built_cert(&cert, input.ttl_policy)?;

    let cert_hash = cert_hash(&cert)?;
    let root_sig = sign_cert_hash(cert_hash).map_err(IssueDelegationProofV2Error::SignFailed)?;

    Ok(IssuedDelegationProofV2 {
        proof: DelegationProofV2 {
            cert,
            root_sig,
            root_public_key_sec1: Some(input.root_public_key_sec1),
            root_key_cert: input.root_key_cert,
        },
        cert_hash,
    })
}

fn validate_cert_issuance_policy_for_built_cert(
    cert: &DelegationCertV2,
    ttl_policy: DelegatedAuthTtlPolicyV2,
) -> Result<(), CertPolicyV2Error> {
    super::policy::validate_cert_issuance_policy(
        cert,
        ttl_policy,
        cert.root_pid,
        cert.shard_key_hash,
    )
}

fn validate_scopes(scopes: &[String]) -> Result<(), IssueDelegationProofV2Error> {
    if scopes.is_empty() {
        return Err(IssueDelegationProofV2Error::ScopesEmpty);
    }
    if scopes.iter().any(String::is_empty) {
        return Err(IssueDelegationProofV2Error::ScopeEmpty);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dto::auth::RootKeyCertificateV2, ids::CanisterRole};

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn ttl_policy() -> DelegatedAuthTtlPolicyV2 {
        DelegatedAuthTtlPolicyV2 {
            max_cert_ttl_secs: 600,
            max_token_ttl_secs: 120,
        }
    }

    fn input() -> IssueDelegationProofV2Input {
        IssueDelegationProofV2Input {
            root_pid: p(1),
            root_key_id: "root-key".to_string(),
            root_public_key_sec1: vec![10, 11, 12],
            root_key_cert: None,
            shard_pid: p(2),
            shard_key_id: "shard-key".to_string(),
            shard_public_key_sec1: vec![20, 21, 22],
            shard_key_binding: ShardKeyBindingV2::IcThresholdEcdsa {
                key_name_hash: [3; 32],
                derivation_path_hash: [4; 32],
            },
            issued_at: 100,
            cert_ttl_secs: 400,
            max_token_ttl_secs: 120,
            scopes: vec!["read".to_string(), "write".to_string()],
            audience: DelegationAudienceV2::Roles(vec![CanisterRole::new("project_instance")]),
            ttl_policy: ttl_policy(),
        }
    }

    #[test]
    fn issue_delegation_proof_v2_signs_exact_cert_hash_and_embeds_root_key() {
        let mut observed_hash = None;

        let issued = issue_delegation_proof_v2(input(), |hash| {
            observed_hash = Some(hash);
            Ok(hash.to_vec())
        })
        .unwrap();

        assert_eq!(issued.proof.cert.version, DELEGATED_AUTH_V2_VERSION);
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
        assert_eq!(issued.proof.root_public_key_sec1, Some(vec![10, 11, 12]));
        assert_eq!(issued.cert_hash, cert_hash(&issued.proof.cert).unwrap());
        assert_eq!(observed_hash, Some(issued.cert_hash));
        assert_eq!(issued.proof.root_sig, issued.cert_hash.to_vec());
    }

    #[test]
    fn issue_delegation_proof_v2_preserves_root_key_certificate() {
        let mut input = input();
        input.root_key_cert = Some(RootKeyCertificateV2 {
            root_pid: p(1),
            key_id: "root-key".to_string(),
            alg: SignatureAlgorithmV2::EcdsaP256Sha256,
            public_key_sec1: vec![10, 11, 12],
            key_hash: public_key_hash(&[10, 11, 12]),
            not_before: 90,
            not_after: None,
            authority_sig: vec![9, 9],
        });

        let issued = issue_delegation_proof_v2(input, |hash| Ok(hash.to_vec())).unwrap();

        assert_eq!(
            issued.proof.root_key_cert.as_ref().map(|cert| &cert.key_id),
            Some(&"root-key".to_string())
        );
    }

    #[test]
    fn issue_delegation_proof_v2_rejects_empty_scopes() {
        let mut input = input();
        input.scopes = vec![];

        assert_eq!(
            issue_delegation_proof_v2(input, |hash| Ok(hash.to_vec())),
            Err(IssueDelegationProofV2Error::ScopesEmpty)
        );
    }

    #[test]
    fn issue_delegation_proof_v2_rejects_policy_ttl_overflow() {
        let mut input = input();
        input.cert_ttl_secs = 601;

        assert_eq!(
            issue_delegation_proof_v2(input, |hash| Ok(hash.to_vec())),
            Err(IssueDelegationProofV2Error::Policy(
                CertPolicyV2Error::CertTtlExceeded {
                    ttl_secs: 601,
                    max_ttl_secs: 600,
                }
            ))
        );
    }

    #[test]
    fn issue_delegation_proof_v2_rejects_multi_role_cert_audience() {
        let mut input = input();
        input.audience = DelegationAudienceV2::Roles(vec![
            CanisterRole::new("project_instance"),
            CanisterRole::new("project_hub"),
        ]);

        assert_eq!(
            issue_delegation_proof_v2(input, |hash| Ok(hash.to_vec())),
            Err(IssueDelegationProofV2Error::Audience(
                AudienceV2Error::RoleAudienceMustBeSingular
            ))
        );
    }

    #[test]
    fn issue_delegation_proof_v2_rejects_signing_failure() {
        assert_eq!(
            issue_delegation_proof_v2(input(), |_| Err("sign failed".to_string())),
            Err(IssueDelegationProofV2Error::SignFailed(
                "sign failed".to_string()
            ))
        );
    }
}
