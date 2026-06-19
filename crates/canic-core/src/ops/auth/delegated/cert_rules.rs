//! Module: ops::auth::delegated::cert_rules
//!
//! Responsibility: validate delegated auth certificate issuance invariants.
//! Does not own: certificate construction, proof verification, or storage.
//! Boundary: pure delegated auth helper shared by root and issuer flows.

use super::{
    audience::{AudienceError, validate_audience_shape, validate_role_grants},
    canonical::issuer_proof_binding_hash,
};
use crate::{cdk::types::Principal, dto::auth::DelegationCert};
use thiserror::Error;

///
/// DelegatedAuthTtlLimits
///
/// Root-configured TTL limits applied to delegated auth certificates and tokens.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegatedAuthTtlLimits {
    pub max_cert_ttl_ns: u64,
    pub max_token_ttl_ns: u64,
}

///
/// CertRuleError
///
/// Typed failure surface for delegated auth certificate rule validation.
///

#[derive(Debug, Eq, Error, PartialEq)]
pub enum CertRuleError {
    #[error("delegated auth cert root pid mismatch (expected {expected}, found {found})")]
    RootPidMismatch {
        expected: Principal,
        found: Principal,
    },
    #[error("delegated auth cert expires_at must be greater than issued_at")]
    InvalidCertWindow,
    #[error("delegated auth cert ttl {ttl_ns}ns exceeds max {max_ttl_ns}ns")]
    CertTtlExceeded { ttl_ns: u64, max_ttl_ns: u64 },
    #[error("delegated auth max token ttl must be greater than zero")]
    TokenTtlZero,
    #[error("delegated auth max token ttl {ttl_ns}ns exceeds max {max_ttl_ns}ns")]
    TokenTtlExceeded { ttl_ns: u64, max_ttl_ns: u64 },
    #[error("delegated auth max token ttl {token_ttl_ns}ns exceeds cert ttl {cert_ttl_ns}ns")]
    TokenTtlOutlivesCert { token_ttl_ns: u64, cert_ttl_ns: u64 },
    #[error("delegated auth issuer proof binding hash mismatch")]
    IssuerProofBindingHashMismatch,
    #[error(transparent)]
    Audience(#[from] AudienceError),
}

pub fn validate_cert_issuance_rules(
    cert: &DelegationCert,
    limits: DelegatedAuthTtlLimits,
    expected_root_pid: Principal,
) -> Result<(), CertRuleError> {
    if cert.root_pid != expected_root_pid {
        return Err(CertRuleError::RootPidMismatch {
            expected: expected_root_pid,
            found: cert.root_pid,
        });
    }

    if cert.not_before_ns < cert.issued_at_ns {
        return Err(CertRuleError::InvalidCertWindow);
    }

    let cert_ttl_ns = cert
        .expires_at_ns
        .checked_sub(cert.not_before_ns)
        .ok_or(CertRuleError::InvalidCertWindow)?;

    if cert_ttl_ns == 0 {
        return Err(CertRuleError::InvalidCertWindow);
    }

    if cert_ttl_ns > limits.max_cert_ttl_ns {
        return Err(CertRuleError::CertTtlExceeded {
            ttl_ns: cert_ttl_ns,
            max_ttl_ns: limits.max_cert_ttl_ns,
        });
    }

    if cert.max_token_ttl_ns == 0 {
        return Err(CertRuleError::TokenTtlZero);
    }

    if cert.max_token_ttl_ns > limits.max_token_ttl_ns {
        return Err(CertRuleError::TokenTtlExceeded {
            ttl_ns: cert.max_token_ttl_ns,
            max_ttl_ns: limits.max_token_ttl_ns,
        });
    }

    if cert.max_token_ttl_ns > cert_ttl_ns {
        return Err(CertRuleError::TokenTtlOutlivesCert {
            token_ttl_ns: cert.max_token_ttl_ns,
            cert_ttl_ns,
        });
    }

    validate_audience_shape(&cert.aud)?;
    validate_role_grants(&cert.grants)?;

    if issuer_proof_binding_hash(
        cert.issuer_pid,
        cert.issuer_proof_alg,
        cert.issuer_proof_binding,
    ) != cert.issuer_proof_binding_hash
    {
        return Err(CertRuleError::IssuerProofBindingHashMismatch);
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
        dto::auth::{
            DelegatedRoleGrant, DelegationAudience, IssuerProofAlgorithm, IssuerProofBinding,
        },
        ids::CanisterRole,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn limits() -> DelegatedAuthTtlLimits {
        DelegatedAuthTtlLimits {
            max_cert_ttl_ns: 600,
            max_token_ttl_ns: 120,
        }
    }

    fn sample_cert() -> DelegationCert {
        let role = CanisterRole::new("project_instance");
        let issuer_proof_alg = IssuerProofAlgorithm::IcCanisterSignatureV1;
        let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [5; 32] };
        let issuer_proof_binding_hash =
            issuer_proof_binding_hash(p(2), issuer_proof_alg, issuer_proof_binding);

        DelegationCert {
            root_pid: p(1),
            issuer_pid: p(2),
            issuer_proof_alg,
            issuer_proof_binding_hash,
            issuer_proof_binding,
            issued_at_ns: 100,
            not_before_ns: 100,
            expires_at_ns: 500,
            max_token_ttl_ns: 120,
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![DelegatedRoleGrant {
                target: role,
                scopes: vec!["read".to_string()],
            }],
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
        cert.expires_at_ns = 900;

        assert_eq!(
            validate_cert_issuance_rules(&cert, limits(), p(1)),
            Err(CertRuleError::CertTtlExceeded {
                ttl_ns: 800,
                max_ttl_ns: 600,
            })
        );
    }

    #[test]
    fn cert_rules_enforce_token_ttl_bound_at_root() {
        let mut cert = sample_cert();
        cert.max_token_ttl_ns = 121;

        assert_eq!(
            validate_cert_issuance_rules(&cert, limits(), p(1)),
            Err(CertRuleError::TokenTtlExceeded {
                ttl_ns: 121,
                max_ttl_ns: 120,
            })
        );
    }

    #[test]
    fn cert_rules_reject_token_ttl_outliving_cert() {
        let mut cert = sample_cert();
        cert.expires_at_ns = 150;

        assert_eq!(
            validate_cert_issuance_rules(&cert, limits(), p(1)),
            Err(CertRuleError::TokenTtlOutlivesCert {
                token_ttl_ns: 120,
                cert_ttl_ns: 50,
            })
        );
    }

    #[test]
    fn cert_rules_enforce_role_grant_shape() {
        let mut cert = sample_cert();
        cert.grants = Vec::new();

        assert_eq!(
            validate_cert_issuance_rules(&cert, limits(), p(1)),
            Err(CertRuleError::Audience(AudienceError::GrantsEmpty))
        );
    }

    #[test]
    fn cert_rules_enforce_issuer_proof_binding_hash() {
        let mut cert = sample_cert();
        cert.issuer_proof_binding_hash = [7; 32];

        assert_eq!(
            validate_cert_issuance_rules(&cert, limits(), p(1)),
            Err(CertRuleError::IssuerProofBindingHashMismatch)
        );
    }
}
