use super::{
    audience::{AudienceError, validate_cert_role_hash},
    canonical::public_key_hash,
};
use crate::{cdk::types::Principal, dto::auth::DelegationCert};
use thiserror::Error;

pub const DELEGATED_AUTH_VERSION: u16 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegatedAuthTtlLimits {
    pub max_cert_ttl_secs: u64,
    pub max_token_ttl_secs: u64,
}

#[derive(Debug, Eq, Error, PartialEq)]
pub enum CertRuleError {
    #[error("delegated auth cert version mismatch (expected {expected}, found {found})")]
    VersionMismatch { expected: u16, found: u16 },
    #[error("delegated auth cert root pid mismatch (expected {expected}, found {found})")]
    RootPidMismatch {
        expected: Principal,
        found: Principal,
    },
    #[error("delegated auth cert expires_at must be greater than issued_at")]
    InvalidCertWindow,
    #[error("delegated auth cert ttl {ttl_secs}s exceeds max {max_ttl_secs}s")]
    CertTtlExceeded { ttl_secs: u64, max_ttl_secs: u64 },
    #[error("delegated auth max token ttl must be greater than zero")]
    TokenTtlZero,
    #[error("delegated auth max token ttl {ttl_secs}s exceeds max {max_ttl_secs}s")]
    TokenTtlExceeded { ttl_secs: u64, max_ttl_secs: u64 },
    #[error("delegated auth max token ttl {token_ttl_secs}s exceeds cert ttl {cert_ttl_secs}s")]
    TokenTtlOutlivesCert {
        token_ttl_secs: u64,
        cert_ttl_secs: u64,
    },
    #[error("delegated auth shard public key hash mismatch")]
    ShardPublicKeyHashMismatch,
    #[error(transparent)]
    Audience(#[from] AudienceError),
}

pub fn validate_cert_issuance_rules(
    cert: &DelegationCert,
    limits: DelegatedAuthTtlLimits,
    expected_root_pid: Principal,
) -> Result<(), CertRuleError> {
    if cert.version != DELEGATED_AUTH_VERSION {
        return Err(CertRuleError::VersionMismatch {
            expected: DELEGATED_AUTH_VERSION,
            found: cert.version,
        });
    }

    if cert.root_pid != expected_root_pid {
        return Err(CertRuleError::RootPidMismatch {
            expected: expected_root_pid,
            found: cert.root_pid,
        });
    }

    let cert_ttl_secs = cert
        .expires_at
        .checked_sub(cert.issued_at)
        .ok_or(CertRuleError::InvalidCertWindow)?;

    if cert_ttl_secs == 0 {
        return Err(CertRuleError::InvalidCertWindow);
    }

    if cert_ttl_secs > limits.max_cert_ttl_secs {
        return Err(CertRuleError::CertTtlExceeded {
            ttl_secs: cert_ttl_secs,
            max_ttl_secs: limits.max_cert_ttl_secs,
        });
    }

    if cert.max_token_ttl_secs == 0 {
        return Err(CertRuleError::TokenTtlZero);
    }

    if cert.max_token_ttl_secs > limits.max_token_ttl_secs {
        return Err(CertRuleError::TokenTtlExceeded {
            ttl_secs: cert.max_token_ttl_secs,
            max_ttl_secs: limits.max_token_ttl_secs,
        });
    }

    if cert.max_token_ttl_secs > cert_ttl_secs {
        return Err(CertRuleError::TokenTtlOutlivesCert {
            token_ttl_secs: cert.max_token_ttl_secs,
            cert_ttl_secs,
        });
    }

    validate_cert_role_hash(&cert.aud, cert.verifier_role_hash)?;

    if public_key_hash(&cert.shard_public_key_sec1) != cert.shard_key_hash {
        return Err(CertRuleError::ShardPublicKeyHashMismatch);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::auth::{DelegationAudience, ShardKeyBinding, SignatureAlgorithm},
        ids::CanisterRole,
        ops::auth::delegated::canonical::role_hash,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn limits() -> DelegatedAuthTtlLimits {
        DelegatedAuthTtlLimits {
            max_cert_ttl_secs: 600,
            max_token_ttl_secs: 120,
        }
    }

    fn sample_cert() -> DelegationCert {
        let role = CanisterRole::new("project_instance");
        let shard_public_key_sec1 = vec![2, 3, 4];
        let shard_key_hash = public_key_hash(&shard_public_key_sec1);

        DelegationCert {
            version: DELEGATED_AUTH_VERSION,
            root_pid: p(1),
            root_key_id: "root-key".to_string(),
            root_key_hash: [9; 32],
            alg: SignatureAlgorithm::EcdsaP256Sha256,
            shard_pid: p(2),
            shard_key_id: "shard-key".to_string(),
            shard_public_key_sec1,
            shard_key_hash,
            shard_key_binding: ShardKeyBinding::IcThresholdEcdsa {
                key_name_hash: [5; 32],
                derivation_path_hash: [6; 32],
            },
            issued_at: 100,
            expires_at: 500,
            max_token_ttl_secs: 120,
            scopes: vec!["read".to_string()],
            aud: DelegationAudience::Roles(vec![role.clone()]),
            verifier_role_hash: Some(role_hash(&role).unwrap()),
        }
    }

    #[test]
    fn cert_rules_accept_well_formed_cert() {
        let cert = sample_cert();

        validate_cert_issuance_rules(&cert, limits(), p(1)).unwrap();
    }

    #[test]
    fn cert_rules_enforce_root_pid_binding() {
        let cert = sample_cert();

        assert_eq!(
            validate_cert_issuance_rules(&cert, limits(), p(9)),
            Err(CertRuleError::RootPidMismatch {
                expected: p(9),
                found: p(1),
            })
        );
    }

    #[test]
    fn cert_rules_enforce_cert_ttl_bound_at_root() {
        let mut cert = sample_cert();
        cert.expires_at = 900;

        assert_eq!(
            validate_cert_issuance_rules(&cert, limits(), p(1)),
            Err(CertRuleError::CertTtlExceeded {
                ttl_secs: 800,
                max_ttl_secs: 600,
            })
        );
    }

    #[test]
    fn cert_rules_enforce_token_ttl_bound_at_root() {
        let mut cert = sample_cert();
        cert.max_token_ttl_secs = 121;

        assert_eq!(
            validate_cert_issuance_rules(&cert, limits(), p(1)),
            Err(CertRuleError::TokenTtlExceeded {
                ttl_secs: 121,
                max_ttl_secs: 120,
            })
        );
    }

    #[test]
    fn cert_rules_reject_token_ttl_outliving_cert() {
        let mut cert = sample_cert();
        cert.expires_at = 150;

        assert_eq!(
            validate_cert_issuance_rules(&cert, limits(), p(1)),
            Err(CertRuleError::TokenTtlOutlivesCert {
                token_ttl_secs: 120,
                cert_ttl_secs: 50,
            })
        );
    }

    #[test]
    fn cert_rules_enforce_role_hash_binding() {
        let mut cert = sample_cert();
        cert.verifier_role_hash = Some([1; 32]);

        assert_eq!(
            validate_cert_issuance_rules(&cert, limits(), p(1)),
            Err(CertRuleError::Audience(AudienceError::RoleHashMismatch))
        );
    }

    #[test]
    fn cert_rules_enforce_shard_public_key_hash_binding() {
        let mut cert = sample_cert();
        cert.shard_public_key_sec1 = vec![7, 8, 9];

        assert_eq!(
            validate_cert_issuance_rules(&cert, limits(), p(1)),
            Err(CertRuleError::ShardPublicKeyHashMismatch)
        );
    }
}
