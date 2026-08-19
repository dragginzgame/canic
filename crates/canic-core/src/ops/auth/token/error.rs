//! Module: ops::auth::token::error
//!
//! Responsibility: project delegated-token proof failures into typed runtime causes and metrics.
//! Does not own: proof verification, token preparation, or public endpoint projection.
//! Boundary: deterministic error and metric-reason mapping for token ops.

use super::*;

pub(super) const fn active_delegation_proof_unavailable_error(
    status: ActiveDelegationProofStatus,
) -> InternalError {
    match status {
        ActiveDelegationProofStatus::Expired => InternalError::auth_proof_expired(),
        ActiveDelegationProofStatus::Missing => {
            InternalError::public(crate::diagnostics::codes::SECURITY_UNAVAILABLE)
        }
        ActiveDelegationProofStatus::RefreshNeeded => InternalError::auth_material_stale(),
        ActiveDelegationProofStatus::Valid => {
            InternalError::public(crate::diagnostics::codes::SECURITY_INVALID_STATE)
        }
    }
}

pub(super) fn map_prepare_delegated_token_error(err: PrepareDelegatedTokenError) -> InternalError {
    match err {
        PrepareDelegatedTokenError::CertNotYetValid => {
            InternalError::public(crate::diagnostics::codes::SECURITY_INVALID_STATE)
        }
        PrepareDelegatedTokenError::CertExpired => InternalError::auth_proof_expired(),
        PrepareDelegatedTokenError::TokenTtlZero
        | PrepareDelegatedTokenError::Audience(_)
        | PrepareDelegatedTokenError::Canonical(_) => {
            InternalError::public(crate::diagnostics::codes::SECURITY_INVALID)
        }
        PrepareDelegatedTokenError::TokenExpiresAtOverflow
        | PrepareDelegatedTokenError::TokenTtlExceeded { .. } => {
            InternalError::public(crate::diagnostics::codes::TIME_CAPACITY)
        }
        PrepareDelegatedTokenError::TokenOutlivesCert => {
            InternalError::public(crate::diagnostics::codes::SECURITY_ORDERING)
        }
        PrepareDelegatedTokenError::AudienceNotSubset
        | PrepareDelegatedTokenError::GrantsNotSubset => {
            InternalError::public(crate::diagnostics::codes::AUTHORITY_INVALID_STATE)
        }
        #[cfg(test)]
        PrepareDelegatedTokenError::IssuerProofFailed(_) => {
            InternalError::public(crate::diagnostics::codes::SECURITY_FAILED)
        }
    }
}

pub(super) trait AuthProofCause: std::fmt::Display {
    fn into_internal_error(self) -> InternalError;
}

impl AuthProofCause for InternalError {
    fn into_internal_error(self) -> InternalError {
        self
    }
}

impl AuthProofCause for String {
    fn into_internal_error(self) -> InternalError {
        InternalError::public(crate::diagnostics::codes::SECURITY_INVALID)
    }
}

pub(super) fn map_verify_delegated_token_error<RootProofError, IssuerProofError>(
    err: VerifyDelegatedTokenError<RootProofError, IssuerProofError>,
) -> InternalError
where
    RootProofError: AuthProofCause,
    IssuerProofError: AuthProofCause,
{
    match err {
        VerifyDelegatedTokenError::CertExpired => InternalError::auth_proof_expired(),
        VerifyDelegatedTokenError::CertNotYetValid
        | VerifyDelegatedTokenError::TokenNotYetValid => {
            InternalError::public(crate::diagnostics::codes::SECURITY_INVALID_STATE)
        }
        VerifyDelegatedTokenError::TokenExpired => InternalError::auth_token_expired(),
        VerifyDelegatedTokenError::IssuerProofUnavailable => InternalError::auth_material_stale(),
        VerifyDelegatedTokenError::RootProofInvalid(cause) => cause.into_internal_error(),
        VerifyDelegatedTokenError::IssuerProofInvalid(cause) => cause.into_internal_error(),
        VerifyDelegatedTokenError::CertHashMismatch => {
            InternalError::public(crate::diagnostics::codes::DIGEST_CONFLICT)
        }
        VerifyDelegatedTokenError::IssuerPidMismatch => {
            InternalError::public(crate::diagnostics::codes::SECURITY_CONFLICT)
        }
        VerifyDelegatedTokenError::PresenterCallerMismatch
        | VerifyDelegatedTokenError::PresenterSubjectMismatch => {
            InternalError::public(crate::diagnostics::codes::AUTHORITY_CONFLICT)
        }
        VerifyDelegatedTokenError::ApplicationAuthority(_)
        | VerifyDelegatedTokenError::TokenInvalidWindow
        | VerifyDelegatedTokenError::Canonical(_)
        | VerifyDelegatedTokenError::CertRules(_)
        | VerifyDelegatedTokenError::Audience(_) => {
            InternalError::public(crate::diagnostics::codes::SECURITY_INVALID)
        }
        VerifyDelegatedTokenError::TokenTtlExceeded { .. } => {
            InternalError::public(crate::diagnostics::codes::TIME_CAPACITY)
        }
        VerifyDelegatedTokenError::TokenIssuedBeforeCert
        | VerifyDelegatedTokenError::TokenOutlivesCert => {
            InternalError::public(crate::diagnostics::codes::SECURITY_ORDERING)
        }
        VerifyDelegatedTokenError::AudienceNotSubset
        | VerifyDelegatedTokenError::GrantsNotSubset => {
            InternalError::public(crate::diagnostics::codes::AUTHORITY_INVALID_STATE)
        }
        VerifyDelegatedTokenError::TokenAudienceRejected
        | VerifyDelegatedTokenError::CertAudienceRejected => {
            InternalError::public(crate::diagnostics::codes::AUTHORITY_INACTIVE)
        }
        VerifyDelegatedTokenError::TokenGrantRejected
        | VerifyDelegatedTokenError::ScopeRejected { .. } => {
            InternalError::public(crate::diagnostics::codes::CONFIGURATION_INACTIVE)
        }
        VerifyDelegatedTokenError::MissingLocalRole => {
            InternalError::public(crate::diagnostics::codes::CONFIGURATION_UNAVAILABLE)
        }
    }
}

// Convert typed verifier failures into bounded metric reasons.
pub(super) const fn delegated_auth_reason_from_verify_error<RootProofError, IssuerProofError>(
    err: &VerifyDelegatedTokenError<RootProofError, IssuerProofError>,
) -> DelegatedAuthMetricReason {
    match err {
        VerifyDelegatedTokenError::Audience(_) => DelegatedAuthMetricReason::Audience,
        VerifyDelegatedTokenError::AudienceNotSubset => {
            DelegatedAuthMetricReason::AudienceNotSubset
        }
        VerifyDelegatedTokenError::ApplicationAuthority(_)
        | VerifyDelegatedTokenError::Canonical(_) => DelegatedAuthMetricReason::Canonical,
        VerifyDelegatedTokenError::CertAudienceRejected => {
            DelegatedAuthMetricReason::CertAudienceRejected
        }
        VerifyDelegatedTokenError::CertExpired => DelegatedAuthMetricReason::CertExpired,
        VerifyDelegatedTokenError::CertHashMismatch => DelegatedAuthMetricReason::CertHashMismatch,
        VerifyDelegatedTokenError::CertNotYetValid => DelegatedAuthMetricReason::CertNotYetValid,
        VerifyDelegatedTokenError::CertRules(_) => DelegatedAuthMetricReason::CertPolicy,
        VerifyDelegatedTokenError::GrantsNotSubset => DelegatedAuthMetricReason::GrantsNotSubset,
        VerifyDelegatedTokenError::IssuerProofInvalid(_) => {
            DelegatedAuthMetricReason::IssuerProofInvalid
        }
        VerifyDelegatedTokenError::IssuerProofUnavailable => {
            DelegatedAuthMetricReason::IssuerProofUnavailable
        }
        VerifyDelegatedTokenError::IssuerPidMismatch => {
            DelegatedAuthMetricReason::IssuerPidMismatch
        }
        VerifyDelegatedTokenError::MissingLocalRole => DelegatedAuthMetricReason::MissingLocalRole,
        VerifyDelegatedTokenError::PresenterCallerMismatch => {
            DelegatedAuthMetricReason::PresenterCallerMismatch
        }
        VerifyDelegatedTokenError::PresenterSubjectMismatch => {
            DelegatedAuthMetricReason::PresenterSubjectMismatch
        }
        VerifyDelegatedTokenError::RootProofInvalid(_) => {
            DelegatedAuthMetricReason::RootProofInvalid
        }
        VerifyDelegatedTokenError::ScopeRejected { .. } => DelegatedAuthMetricReason::ScopeRejected,
        VerifyDelegatedTokenError::TokenAudienceRejected => {
            DelegatedAuthMetricReason::TokenAudienceRejected
        }
        VerifyDelegatedTokenError::TokenExpired => DelegatedAuthMetricReason::TokenExpired,
        VerifyDelegatedTokenError::TokenGrantRejected => {
            DelegatedAuthMetricReason::TokenGrantRejected
        }
        VerifyDelegatedTokenError::TokenInvalidWindow => {
            DelegatedAuthMetricReason::TokenInvalidWindow
        }
        VerifyDelegatedTokenError::TokenIssuedBeforeCert => {
            DelegatedAuthMetricReason::TokenIssuedBeforeCert
        }
        VerifyDelegatedTokenError::TokenNotYetValid => DelegatedAuthMetricReason::TokenNotYetValid,
        VerifyDelegatedTokenError::TokenOutlivesCert => {
            DelegatedAuthMetricReason::TokenOutlivesCert
        }
        VerifyDelegatedTokenError::TokenTtlExceeded { .. } => {
            DelegatedAuthMetricReason::TokenTtlExceeded
        }
    }
}
