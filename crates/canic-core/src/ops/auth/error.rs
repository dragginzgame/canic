//! Module: ops::auth::error
//!
//! Responsibility: define typed auth operation failure surfaces.
//! Does not own: public error DTOs, endpoint mapping, or verification logic.
//! Boundary: converts auth-local failures into internal errors.

use crate::{InternalError, ops::prelude::*};
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

    #[error("local-Subnet attestation requires an explicit subnet claim")]
    AttestationSubnetRequired,

    #[error("attestation proof field '{field}' exceeds {max_bytes} bytes (found {actual_bytes})")]
    AttestationProofFieldTooLarge {
        field: &'static str,
        actual_bytes: usize,
        max_bytes: usize,
    },

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
    #[error("token issuer pid mismatch (expected {expected}, found {found})")]
    IssuerPidMismatch {
        expected: Principal,
        found: Principal,
    },

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
        match err {
            AuthOpsError::Validation(err) => err.into(),
            AuthOpsError::Signature(err) => err.into(),
            AuthOpsError::Scope(err) => err.into(),
            AuthOpsError::Expiry(err) => err.into(),
        }
    }
}

impl From<AuthValidationError> for InternalError {
    fn from(err: AuthValidationError) -> Self {
        use crate::diagnostics::codes;

        let code = match err {
            AuthValidationError::InvalidRootAuthority { .. } => codes::AUTHORITY_CONFLICT,
            AuthValidationError::EncodeFailed { .. } => codes::CODEC_FAILED,
            AuthValidationError::AttestationSubnetUnavailable
            | AuthValidationError::AttestationSubnetRequired => codes::AUTHORITY_UNAVAILABLE,
            AuthValidationError::AttestationProofFieldTooLarge { .. } => {
                codes::SECURITY_INVALID_STATE
            }
            AuthValidationError::AttestationInvalidWindow { .. } | AuthValidationError::Auth(_) => {
                codes::SECURITY_INVALID
            }
            AuthValidationError::DelegatedTokenAuthDisabled => codes::SECURITY_INACTIVE,
        };
        Self::public(code)
    }
}

impl From<AuthSignatureError> for InternalError {
    fn from(err: AuthSignatureError) -> Self {
        match err {
            AuthSignatureError::ProofUnavailable => {
                Self::public(crate::diagnostics::codes::SECURITY_UNAVAILABLE)
            }
            AuthSignatureError::ProofInvalid(_)
            | AuthSignatureError::AttestationProofInvalid(_) => {
                Self::public(crate::diagnostics::codes::SECURITY_INVALID)
            }
            AuthSignatureError::RootDataCertificateUnavailable => {
                Self::root_data_certificate_unavailable()
            }
        }
    }
}

impl From<AuthScopeError> for InternalError {
    fn from(_err: AuthScopeError) -> Self {
        Self::public(crate::diagnostics::codes::AUTHORITY_CONFLICT)
    }
}

impl From<AuthExpiryError> for InternalError {
    fn from(err: AuthExpiryError) -> Self {
        use crate::diagnostics::codes;

        let code = match err {
            AuthExpiryError::CertExpired { .. } => codes::AUTH_CERT_EXPIRED,
            AuthExpiryError::TokenExpired { .. } => codes::AUTH_TOKEN_EXPIRED,
            AuthExpiryError::TokenNotYetValid { .. }
            | AuthExpiryError::AttestationNotYetValid { .. } => codes::SECURITY_INVALID_STATE,
            AuthExpiryError::TokenTtlExceeded { .. } => codes::TIME_CAPACITY,
            AuthExpiryError::AttestationExpired { .. } => codes::SECURITY_EXPIRED,
            AuthExpiryError::AttestationEpochRejected { .. } => codes::VERSION_INACTIVE,
        };
        Self::public(code)
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
        let public = err.public_error();

        assert_eq!(
            public.code(),
            crate::diagnostics::codes::SECURITY_UNAVAILABLE.raw_code()
        );
    }

    #[test]
    fn proof_availability_and_validation_retain_public_auth_causes() {
        let unavailable: InternalError = AuthSignatureError::ProofUnavailable.into();
        let invalid: InternalError =
            AuthSignatureError::ProofInvalid("bad signature".to_string()).into();

        assert_eq!(
            unavailable.public_error().code(),
            crate::diagnostics::codes::SECURITY_UNAVAILABLE.raw_code()
        );
        assert_eq!(
            invalid.public_error().code(),
            crate::diagnostics::codes::SECURITY_INVALID.raw_code()
        );
    }
}
