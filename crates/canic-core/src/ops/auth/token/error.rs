//! Module: ops::auth::token::error
//!
//! Responsibility: project delegated-token proof failures into typed runtime causes and metrics.
//! Does not own: proof verification, token preparation, or public endpoint projection.
//! Boundary: deterministic error and metric-reason mapping for token ops.

use super::*;

///
/// DelegatedTokenIssuerPrepareError
///
/// Preserves missing issuer material separately from other public E10 failures.
///
pub enum DelegatedTokenIssuerPrepareError {
    MissingProof,
    Failed(InternalError),
}

impl From<InternalError> for DelegatedTokenIssuerPrepareError {
    fn from(error: InternalError) -> Self {
        Self::Failed(error)
    }
}

impl DelegatedTokenIssuerPrepareError {
    pub(crate) fn into_internal_error(self) -> InternalError {
        match self {
            Self::MissingProof => {
                active_delegation_proof_unavailable_error(ActiveDelegationProofStatus::Missing)
            }
            Self::Failed(error) => error,
        }
    }
}

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
        | PrepareDelegatedTokenError::GrantsEmpty
        | PrepareDelegatedTokenError::TooManyGrants { .. }
        | PrepareDelegatedTokenError::EmptyGrantScopes { .. }
        | PrepareDelegatedTokenError::TooManyGrantScopes { .. }
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
    }
}
