//! Module: ops::auth::delegated::cert_rules
//!
//! Responsibility: project Canic certificate inputs and protected bounds into IC Auth.
//! Does not own: certificate rules, signing, proof verification or storage.
//! Boundary: retain Canic role projection failures separately from upstream issuance refusals.

use super::{
    audience::{MAX_DELEGATED_ROLE_GRANTS, MAX_SCOPES_PER_ROLE_GRANT},
    canonical::CanonicalAuthError,
    protocol,
};
use crate::{cdk::types::Principal, dto::auth::DelegationCert};
use ic_auth::certificate_issuance::{
    CertificateIssuanceError, CertificateIssuanceLimits, validate_certificate_issuance,
};
use thiserror::Error;

/// Root-configured certificate and token lifetime ceilings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegatedAuthTtlLimits {
    pub max_cert_ttl_ns: u64,
    pub max_token_ttl_ns: u64,
}

/// Canic identity conversion or the shared certificate owner's typed refusal.
#[derive(Debug, Eq, Error, PartialEq)]
pub enum CertificateIssuanceValidationError {
    #[error(transparent)]
    Projection(#[from] CanonicalAuthError),
    #[error(transparent)]
    Rules(#[from] CertificateIssuanceError),
}

/// Validate exact projected bytes with explicit protected Root and resource bounds.
pub fn validate_cert_issuance_rules(
    cert: &DelegationCert,
    limits: DelegatedAuthTtlLimits,
    expected_root_pid: Principal,
) -> Result<(), CertificateIssuanceValidationError> {
    validate_certificate_issuance(
        &protocol::cert(cert)?,
        expected_root_pid.into(),
        CertificateIssuanceLimits {
            max_cert_ttl_ns: limits.max_cert_ttl_ns,
            max_token_ttl_ns: limits.max_token_ttl_ns,
            max_grants: MAX_DELEGATED_ROLE_GRANTS,
            max_scopes_per_grant: MAX_SCOPES_PER_ROLE_GRANT,
        },
    )?;
    Ok(())
}
