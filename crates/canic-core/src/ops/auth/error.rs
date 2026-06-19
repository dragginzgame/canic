//! Module: ops::auth::error
//!
//! Responsibility: define typed auth operation failure surfaces.
//! Does not own: public error DTOs, endpoint mapping, or verification logic.
//! Boundary: converts auth-local failures into internal errors.

use crate::{InternalError, InternalErrorOrigin, ids::CanisterRole, ops::prelude::*};
use thiserror::Error as ThisError;

///
/// AuthOpsError
///
/// Aggregate typed failure surface for auth operations.
///

#[derive(Debug, ThisError)]
pub enum AuthOpsError {
    #[error(transparent)]
    Validation(#[from] AuthValidationError),

    #[error(transparent)]
    Signature(#[from] AuthSignatureError),

    #[error(transparent)]
    Scope(#[from] AuthScopeError),

    #[error(transparent)]
    Expiry(#[from] AuthExpiryError),
}

///
/// AuthValidationError
///
/// Typed failure surface for auth input and canonical encoding validation.
///

#[derive(Debug, ThisError)]
pub enum AuthValidationError {
    #[error(
        "delegation cert expires_at ({expires_at}) must be greater than issued_at ({issued_at})"
    )]
    CertInvalidWindow { issued_at: u64, expires_at: u64 },

    #[error("delegation cert root pid mismatch (expected {expected}, found {found})")]
    InvalidRootAuthority {
        expected: Principal,
        found: Principal,
    },

    #[error("candid encode failed for {context}: {source}")]
    EncodeFailed {
        context: &'static str,
        source: candid::Error,
    },

    #[error("attestation subnet was set but verifier subnet is unavailable")]
    AttestationSubnetUnavailable,

    #[error(
        "attestation expires_at_ns ({expires_at_ns}) must be greater than issued_at_ns ({issued_at_ns})"
    )]
    AttestationInvalidWindow {
        issued_at_ns: u64,
        expires_at_ns: u64,
    },

    #[error("delegated token auth disabled (set auth.delegated_tokens.enabled=true in canic.toml)")]
    DelegatedTokenAuthDisabled,

    #[error("auth validation failed: {0}")]
    Auth(String),
}

///
/// AuthSignatureError
///
/// Typed failure surface for auth proof availability and signature validation.
///

#[derive(Debug, ThisError)]
pub enum AuthSignatureError {
    #[error("auth proof unavailable")]
    ProofUnavailable,

    #[error("auth proof invalid: {0}")]
    ProofInvalid(String),

    #[error("root data certificate unavailable")]
    RootDataCertificateUnavailable,

    #[error("attestation proof invalid: {0}")]
    AttestationProofInvalid(String),
}

///
/// AuthScopeError
///
/// Typed failure surface for delegated auth audience, issuer, and scope checks.
///

#[derive(Debug, ThisError)]
pub enum AuthScopeError {
    #[error("audience principal '{aud}' not allowed by delegation")]
    AudienceNotAllowed { aud: Principal },

    #[error("audience role '{role}' not allowed by delegation")]
    AudienceRoleNotAllowed { role: CanisterRole },

    #[error("wildcard verifier audience not allowed by role-scoped delegation")]
    AudienceAnyNotAllowed,

    #[error("token audience role list must not be empty")]
    AudienceRoleListEmpty,

    #[error("scope '{scope}' not allowed by delegation")]
    ScopeNotAllowed { scope: String },

    #[error("token issuer pid mismatch (expected {expected}, found {found})")]
    IssuerPidMismatch {
        expected: Principal,
        found: Principal,
    },

    #[error("token audience does not include local canister '{self_pid}'")]
    SelfAudienceMissing { self_pid: Principal },

    #[error("token audience does not include local canister role '{role}' for '{self_pid}'")]
    SelfRoleAudienceMissing {
        self_pid: Principal,
        role: CanisterRole,
    },

    #[error("local canister '{self_pid}' is not configured as a delegated auth verifier")]
    SelfVerifierUnavailable { self_pid: Principal },

    #[error("attestation subject mismatch (expected caller {expected}, found {found})")]
    AttestationSubjectMismatch {
        expected: Principal,
        found: Principal,
    },

    #[error("attestation audience mismatch (expected {expected}, found {found})")]
    AttestationAudienceMismatch {
        expected: Principal,
        found: Principal,
    },

    #[error("attestation subnet mismatch (expected {expected}, found {found})")]
    AttestationSubnetMismatch {
        expected: Principal,
        found: Principal,
    },
}

///
/// AuthExpiryError
///
/// Typed failure surface for auth proof and token time-window checks.
///

#[derive(Debug, ThisError)]
pub enum AuthExpiryError {
    #[error("delegation cert expired at {expires_at}")]
    CertExpired { expires_at: u64 },

    #[error("token expired at {exp}")]
    TokenExpired { exp: u64 },

    #[error("token not yet valid (iat {iat})")]
    TokenNotYetValid { iat: u64 },

    #[error("token issued before delegation (iat {token_iat} < cert {cert_iat})")]
    TokenIssuedBeforeDelegation { token_iat: u64, cert_iat: u64 },

    #[error("token expires after delegation (exp {token_exp} > cert {cert_exp})")]
    TokenOutlivesDelegation { token_exp: u64, cert_exp: u64 },

    #[error("delegated token expiry precedes issued_at")]
    TokenExpiryBeforeIssued,

    #[error("delegated token ttl exceeds max {max_ttl_secs}s (ttl {ttl_secs}s)")]
    TokenTtlExceeded { ttl_secs: u64, max_ttl_secs: u64 },

    #[error("attestation expired at {expires_at_ns} (now {now_ns})")]
    AttestationExpired { expires_at_ns: u64, now_ns: u64 },

    #[error("attestation not yet valid (issued_at_ns {issued_at_ns}, now {now_ns})")]
    AttestationNotYetValid { issued_at_ns: u64, now_ns: u64 },

    #[error("attestation epoch {epoch} below minimum accepted epoch {min_accepted_epoch}")]
    AttestationEpochRejected { epoch: u64, min_accepted_epoch: u64 },
}

impl From<AuthOpsError> for InternalError {
    fn from(err: AuthOpsError) -> Self {
        Self::ops(InternalErrorOrigin::Ops, err.to_string())
    }
}

impl From<AuthValidationError> for InternalError {
    fn from(err: AuthValidationError) -> Self {
        AuthOpsError::from(err).into()
    }
}

impl From<AuthSignatureError> for InternalError {
    fn from(err: AuthSignatureError) -> Self {
        match err {
            AuthSignatureError::RootDataCertificateUnavailable => {
                Self::root_data_certificate_unavailable()
            }
            _ => AuthOpsError::from(err).into(),
        }
    }
}

impl From<AuthScopeError> for InternalError {
    fn from(err: AuthScopeError) -> Self {
        AuthOpsError::from(err).into()
    }
}

impl From<AuthExpiryError> for InternalError {
    fn from(err: AuthExpiryError) -> Self {
        AuthOpsError::from(err).into()
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_data_certificate_unavailable_maps_to_public_code() {
        let err: InternalError = AuthSignatureError::RootDataCertificateUnavailable.into();
        let public = err
            .public_error()
            .expect("missing root data certificate must be public");

        assert_eq!(
            format!("{:?}", public.code),
            "RootDataCertificateUnavailable"
        );
    }
}
