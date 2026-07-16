//! Module: ops::auth::token::error
//!
//! Responsibility: project delegated-token proof failures into typed runtime causes and metrics.
//! Does not own: proof verification, token preparation, or public endpoint projection.
//! Boundary: deterministic error and metric-reason mapping for token ops.

use super::*;

pub(super) fn active_delegation_proof_unavailable_error(now_ns: u64) -> InternalError {
    let status = AuthOps::active_delegation_proof_status(now_ns).status;
    match status {
        ActiveDelegationProofStatus::Expired => InternalError::auth_proof_expired(
            "active delegation proof expired; reprovision auth proof",
        ),
        ActiveDelegationProofStatus::Missing => InternalError::auth_material_stale(
            "active delegation proof is unavailable; provision auth proof",
        ),
        ActiveDelegationProofStatus::RefreshNeeded | ActiveDelegationProofStatus::Valid => {
            InternalError::auth_material_stale(
                "active delegation proof is unavailable or stale; reprovision auth proof",
            )
        }
    }
}

pub(super) fn map_prepare_delegated_token_error(err: PrepareDelegatedTokenError) -> InternalError {
    match err {
        PrepareDelegatedTokenError::CertExpired => InternalError::auth_proof_expired(
            "active delegation proof expired; reprovision auth proof",
        ),
        PrepareDelegatedTokenError::TokenOutlivesCert => InternalError::auth_material_stale(
            "active delegation proof is too close to expiry; reprovision auth proof",
        ),
        err => AuthValidationError::Auth(err.to_string()).into(),
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
        InternalError::invalid_input(self)
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
        err @ VerifyDelegatedTokenError::CertExpired => {
            InternalError::auth_proof_expired(err.to_string())
        }
        err @ VerifyDelegatedTokenError::CertNotYetValid => {
            InternalError::auth_proof_pending(err.to_string())
        }
        err @ VerifyDelegatedTokenError::TokenExpired => {
            InternalError::auth_token_expired(err.to_string())
        }
        VerifyDelegatedTokenError::IssuerProofUnavailable => {
            InternalError::auth_material_stale("delegated auth issuer proof unavailable")
        }
        VerifyDelegatedTokenError::RootProofInvalid(cause) => cause.into_internal_error(),
        VerifyDelegatedTokenError::IssuerProofInvalid(cause) => cause.into_internal_error(),
        err => InternalError::invalid_input(err.to_string()),
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
        VerifyDelegatedTokenError::Canonical(_) => DelegatedAuthMetricReason::Canonical,
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
