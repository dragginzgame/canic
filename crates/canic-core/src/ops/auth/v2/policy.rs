use super::{
    audience::{AudienceV2Error, validate_cert_role_hash},
    canonical::public_key_hash,
};
use crate::{cdk::types::Principal, dto::auth::DelegationCertV2};
use thiserror::Error;

pub const DELEGATED_AUTH_V2_VERSION: u16 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegatedAuthTtlPolicyV2 {
    pub max_cert_ttl_secs: u64,
    pub max_token_ttl_secs: u64,
}

#[derive(Debug, Eq, Error, PartialEq)]
pub enum CertPolicyV2Error {
    #[error("delegated auth v2 cert version mismatch (expected {expected}, found {found})")]
    VersionMismatch { expected: u16, found: u16 },
    #[error("delegated auth v2 cert root pid mismatch (expected {expected}, found {found})")]
    RootPidMismatch {
        expected: Principal,
        found: Principal,
    },
    #[error("delegated auth v2 cert expires_at must be greater than issued_at")]
    InvalidCertWindow,
    #[error("delegated auth v2 cert ttl {ttl_secs}s exceeds max {max_ttl_secs}s")]
    CertTtlExceeded { ttl_secs: u64, max_ttl_secs: u64 },
    #[error("delegated auth v2 max token ttl must be greater than zero")]
    TokenTtlZero,
    #[error("delegated auth v2 max token ttl {ttl_secs}s exceeds max {max_ttl_secs}s")]
    TokenTtlExceeded { ttl_secs: u64, max_ttl_secs: u64 },
    #[error("delegated auth v2 max token ttl {token_ttl_secs}s exceeds cert ttl {cert_ttl_secs}s")]
    TokenTtlOutlivesCert {
        token_ttl_secs: u64,
        cert_ttl_secs: u64,
    },
    #[error("delegated auth v2 shard public key hash mismatch")]
    ShardPublicKeyHashMismatch,
    #[error("delegated auth v2 shard derived key hash mismatch")]
    ShardDerivedKeyHashMismatch,
    #[error(transparent)]
    Audience(#[from] AudienceV2Error),
}

pub fn validate_cert_issuance_policy(
    cert: &DelegationCertV2,
    policy: DelegatedAuthTtlPolicyV2,
    expected_root_pid: Principal,
    expected_shard_key_hash: [u8; 32],
) -> Result<(), CertPolicyV2Error> {
    if cert.version != DELEGATED_AUTH_V2_VERSION {
        return Err(CertPolicyV2Error::VersionMismatch {
            expected: DELEGATED_AUTH_V2_VERSION,
            found: cert.version,
        });
    }

    if cert.root_pid != expected_root_pid {
        return Err(CertPolicyV2Error::RootPidMismatch {
            expected: expected_root_pid,
            found: cert.root_pid,
        });
    }

    let cert_ttl_secs = cert
        .expires_at
        .checked_sub(cert.issued_at)
        .ok_or(CertPolicyV2Error::InvalidCertWindow)?;

    if cert_ttl_secs == 0 {
        return Err(CertPolicyV2Error::InvalidCertWindow);
    }

    if cert_ttl_secs > policy.max_cert_ttl_secs {
        return Err(CertPolicyV2Error::CertTtlExceeded {
            ttl_secs: cert_ttl_secs,
            max_ttl_secs: policy.max_cert_ttl_secs,
        });
    }

    if cert.max_token_ttl_secs == 0 {
        return Err(CertPolicyV2Error::TokenTtlZero);
    }

    if cert.max_token_ttl_secs > policy.max_token_ttl_secs {
        return Err(CertPolicyV2Error::TokenTtlExceeded {
            ttl_secs: cert.max_token_ttl_secs,
            max_ttl_secs: policy.max_token_ttl_secs,
        });
    }

    if cert.max_token_ttl_secs > cert_ttl_secs {
        return Err(CertPolicyV2Error::TokenTtlOutlivesCert {
            token_ttl_secs: cert.max_token_ttl_secs,
            cert_ttl_secs,
        });
    }

    validate_cert_role_hash(&cert.aud, cert.verifier_role_hash)?;

    if public_key_hash(&cert.shard_public_key_sec1) != cert.shard_key_hash {
        return Err(CertPolicyV2Error::ShardPublicKeyHashMismatch);
    }

    if cert.shard_key_hash != expected_shard_key_hash {
        return Err(CertPolicyV2Error::ShardDerivedKeyHashMismatch);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::auth::{DelegationAudienceV2, ShardKeyBindingV2, SignatureAlgorithmV2},
        ids::CanisterRole,
        ops::auth::v2::canonical::role_hash,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn policy() -> DelegatedAuthTtlPolicyV2 {
        DelegatedAuthTtlPolicyV2 {
            max_cert_ttl_secs: 600,
            max_token_ttl_secs: 120,
        }
    }

    fn sample_cert() -> DelegationCertV2 {
        let role = CanisterRole::new("project_instance");
        let shard_public_key_sec1 = vec![2, 3, 4];
        let shard_key_hash = public_key_hash(&shard_public_key_sec1);

        DelegationCertV2 {
            version: DELEGATED_AUTH_V2_VERSION,
            root_pid: p(1),
            root_key_id: "root-key".to_string(),
            root_key_hash: [9; 32],
            alg: SignatureAlgorithmV2::EcdsaP256Sha256,
            shard_pid: p(2),
            shard_key_id: "shard-key".to_string(),
            shard_public_key_sec1,
            shard_key_hash,
            shard_key_binding: ShardKeyBindingV2::IcThresholdEcdsa {
                key_name_hash: [5; 32],
                derivation_path_hash: [6; 32],
            },
            issued_at: 100,
            expires_at: 500,
            max_token_ttl_secs: 120,
            scopes: vec!["read".to_string()],
            aud: DelegationAudienceV2::Roles(vec![role.clone()]),
            verifier_role_hash: Some(role_hash(&role).unwrap()),
        }
    }

    #[test]
    fn cert_policy_accepts_well_formed_cert() {
        let cert = sample_cert();

        validate_cert_issuance_policy(&cert, policy(), p(1), cert.shard_key_hash).unwrap();
    }

    #[test]
    fn cert_policy_enforces_root_pid_binding() {
        let cert = sample_cert();

        assert_eq!(
            validate_cert_issuance_policy(&cert, policy(), p(9), cert.shard_key_hash),
            Err(CertPolicyV2Error::RootPidMismatch {
                expected: p(9),
                found: p(1),
            })
        );
    }

    #[test]
    fn cert_policy_enforces_cert_ttl_bound_at_root() {
        let mut cert = sample_cert();
        cert.expires_at = 900;

        assert_eq!(
            validate_cert_issuance_policy(&cert, policy(), p(1), cert.shard_key_hash),
            Err(CertPolicyV2Error::CertTtlExceeded {
                ttl_secs: 800,
                max_ttl_secs: 600,
            })
        );
    }

    #[test]
    fn cert_policy_enforces_token_ttl_bound_at_root() {
        let mut cert = sample_cert();
        cert.max_token_ttl_secs = 121;

        assert_eq!(
            validate_cert_issuance_policy(&cert, policy(), p(1), cert.shard_key_hash),
            Err(CertPolicyV2Error::TokenTtlExceeded {
                ttl_secs: 121,
                max_ttl_secs: 120,
            })
        );
    }

    #[test]
    fn cert_policy_rejects_token_ttl_outliving_cert() {
        let mut cert = sample_cert();
        cert.expires_at = 150;

        assert_eq!(
            validate_cert_issuance_policy(&cert, policy(), p(1), cert.shard_key_hash),
            Err(CertPolicyV2Error::TokenTtlOutlivesCert {
                token_ttl_secs: 120,
                cert_ttl_secs: 50,
            })
        );
    }

    #[test]
    fn cert_policy_enforces_role_hash_binding() {
        let mut cert = sample_cert();
        cert.verifier_role_hash = Some([1; 32]);

        assert_eq!(
            validate_cert_issuance_policy(&cert, policy(), p(1), cert.shard_key_hash),
            Err(CertPolicyV2Error::Audience(
                AudienceV2Error::RoleHashMismatch
            ))
        );
    }

    #[test]
    fn cert_policy_enforces_shard_public_key_hash_binding() {
        let mut cert = sample_cert();
        let expected = cert.shard_key_hash;
        cert.shard_public_key_sec1 = vec![7, 8, 9];

        assert_eq!(
            validate_cert_issuance_policy(&cert, policy(), p(1), expected),
            Err(CertPolicyV2Error::ShardPublicKeyHashMismatch)
        );
    }

    #[test]
    fn cert_policy_enforces_shard_derivation_binding() {
        let cert = sample_cert();

        assert_eq!(
            validate_cert_issuance_policy(&cert, policy(), p(1), [0; 32]),
            Err(CertPolicyV2Error::ShardDerivedKeyHashMismatch)
        );
    }
}
