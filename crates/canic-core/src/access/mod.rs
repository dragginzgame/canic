//! Module: access
//!
//! Responsibility: compose endpoint access predicates and normalize access denial errors.
//! Does not own: endpoint response mapping, workflow authorization, or runtime metrics storage.
//! Boundary: endpoint macros call access predicates before delegating to workflow.

pub mod auth;
pub mod deployment;
pub mod env;
#[doc(hidden)]
pub mod expr;
pub mod fleet;
pub mod metrics;

use crate::InternalError;
use crate::diagnostics::{RegisteredDiagnosticCode, codes};
use thiserror::Error as ThisError;

///
/// AccessError
///
/// Framework-agnostic access-layer error returned by endpoint access predicates.
/// Each direct denial retains its exact typed meaning until the public boundary.
///

#[derive(Debug, ThisError)]
pub enum AccessError {
    #[error("access denied: an active Component member is required")]
    ActiveComponentRequired,

    #[error("access denied: this endpoint requires a matching build network")]
    BuildNetworkMismatch,

    #[error("access denied: the build network is unavailable")]
    BuildNetworkUnavailable,

    #[error("access denied: a controller is required")]
    ControllerRequired,

    #[error("access denied: delegated auth cert expired")]
    DelegatedAuthCertExpired,

    #[error("access denied: delegated auth token expired")]
    DelegatedAuthTokenExpired,

    #[error("access denied: delegated token authentication is disabled")]
    DelegatedTokensDisabled,

    #[error("access denied: delegated token is malformed")]
    DelegatedTokenMalformed,

    #[error("access denied: delegated token subject does not match caller")]
    DelegatedTokenSubjectMismatch,

    #[error("access denied: a direct child is required")]
    DirectChildRequired,

    #[error("access denied: the Fleet is disabled")]
    FleetDisabled,

    #[error("access denied: the Fleet is read-only")]
    FleetReadonly,

    #[error("access denied: a Fleet Subnet Root is required")]
    FleetSubnetRootRequired,

    #[error(transparent)]
    Internal(InternalError),

    #[error("access denied: a configured parent is required")]
    ParentRequired,

    #[error("access denied: a configured access expression rule is required")]
    ExpressionRuleRequired,

    #[error("access denied: a negated access predicate matched")]
    NegatedPredicateMatched,

    #[error("access denied: the delegated token is missing the required scope")]
    RequiredScopeMissing,

    #[error("access denied: a role attestation is malformed")]
    RoleAttestationMalformed,

    #[error("access denied: role attestation subject does not match caller")]
    RoleAttestationSubjectMismatch,

    #[error("access denied: a Fleet Subnet Root or active Component member is required")]
    RootOrActiveComponentRequired,

    #[error("access denied: a configured root is required")]
    RootRequired,

    #[error("access denied: the current canister is required")]
    SelfRequired,

    #[error("access denied: the Fleet-service access guard is invalid")]
    ServiceGuardInvalid,

    #[error("access denied: the Fleet-service Authority is required")]
    ServiceAuthorityRequired,

    #[error("access denied: delegated token TTL configuration overflows nanoseconds")]
    DelegatedTokenMaxTtlOverflow,

    #[error("access denied: a whitelisted caller is required")]
    WhitelistRequired,
}

impl AccessError {
    #[must_use]
    pub(crate) const fn diagnostic_codes(&self) -> Option<AccessDiagnosticCodes> {
        let codes = match self {
            Self::ActiveComponentRequired
            | Self::ControllerRequired
            | Self::DirectChildRequired
            | Self::FleetSubnetRootRequired
            | Self::ParentRequired
            | Self::RootOrActiveComponentRequired
            | Self::RootRequired
            | Self::SelfRequired
            | Self::ServiceAuthorityRequired => {
                AccessDiagnosticCodes::public(codes::AUTHORITY_UNAVAILABLE)
            }
            Self::BuildNetworkMismatch => AccessDiagnosticCodes::public(codes::PLATFORM_CONFLICT),
            Self::BuildNetworkUnavailable => {
                AccessDiagnosticCodes::public(codes::PLATFORM_UNAVAILABLE)
            }
            Self::DelegatedAuthCertExpired => {
                AccessDiagnosticCodes::public(codes::AUTH_CERT_EXPIRED)
            }
            Self::DelegatedAuthTokenExpired => {
                AccessDiagnosticCodes::public(codes::AUTH_TOKEN_EXPIRED)
            }
            Self::DelegatedTokensDisabled => {
                AccessDiagnosticCodes::public(codes::SECURITY_INACTIVE)
            }
            Self::DelegatedTokenMalformed | Self::RoleAttestationMalformed => {
                AccessDiagnosticCodes::public(codes::SECURITY_INVALID_STATE)
            }
            Self::DelegatedTokenSubjectMismatch | Self::RoleAttestationSubjectMismatch => {
                AccessDiagnosticCodes::public(codes::AUTHORITY_CONFLICT)
            }
            Self::FleetDisabled => AccessDiagnosticCodes::public(codes::AUTHORITY_INACTIVE),
            Self::FleetReadonly => AccessDiagnosticCodes::public(codes::AUTHORITY_INVALID_STATE),
            Self::ExpressionRuleRequired => AccessDiagnosticCodes::projected(
                codes::CONFIGURATION_UNAVAILABLE,
                codes::CONFIGURATION_INVALID,
            ),
            Self::NegatedPredicateMatched => {
                AccessDiagnosticCodes::public(codes::CONFIGURATION_INVALID_STATE)
            }
            Self::RequiredScopeMissing | Self::WhitelistRequired => {
                AccessDiagnosticCodes::public(codes::CONFIGURATION_UNAVAILABLE)
            }
            Self::ServiceGuardInvalid => AccessDiagnosticCodes::projected(
                codes::AUTHORITY_INVALID,
                codes::CONFIGURATION_INVALID,
            ),
            Self::DelegatedTokenMaxTtlOverflow => {
                AccessDiagnosticCodes::projected(codes::TIME_CAPACITY, codes::CONFIGURATION_INVALID)
            }
            Self::Internal(_) => return None,
        };
        Some(codes)
    }
}

///
/// AccessDiagnosticCodes
///
/// Exact and safe public registered identities for one typed access rejection.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AccessDiagnosticCodes {
    pub(crate) exact: RegisteredDiagnosticCode,
    pub(crate) public: RegisteredDiagnosticCode,
}

impl AccessDiagnosticCodes {
    const fn public(code: RegisteredDiagnosticCode) -> Self {
        Self {
            exact: code,
            public: code,
        }
    }

    const fn projected(exact: RegisteredDiagnosticCode, public: RegisteredDiagnosticCode) -> Self {
        Self { exact, public }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_access_rejections_use_the_approved_registered_identities() {
        let public = [
            (
                AccessError::ActiveComponentRequired,
                codes::AUTHORITY_UNAVAILABLE,
            ),
            (AccessError::BuildNetworkMismatch, codes::PLATFORM_CONFLICT),
            (
                AccessError::BuildNetworkUnavailable,
                codes::PLATFORM_UNAVAILABLE,
            ),
            (
                AccessError::ControllerRequired,
                codes::AUTHORITY_UNAVAILABLE,
            ),
            (
                AccessError::DelegatedAuthCertExpired,
                codes::AUTH_CERT_EXPIRED,
            ),
            (
                AccessError::DelegatedAuthTokenExpired,
                codes::AUTH_TOKEN_EXPIRED,
            ),
            (
                AccessError::DelegatedTokensDisabled,
                codes::SECURITY_INACTIVE,
            ),
            (
                AccessError::DelegatedTokenMalformed,
                codes::SECURITY_INVALID_STATE,
            ),
            (
                AccessError::DelegatedTokenSubjectMismatch,
                codes::AUTHORITY_CONFLICT,
            ),
            (
                AccessError::DirectChildRequired,
                codes::AUTHORITY_UNAVAILABLE,
            ),
            (AccessError::FleetDisabled, codes::AUTHORITY_INACTIVE),
            (AccessError::FleetReadonly, codes::AUTHORITY_INVALID_STATE),
            (
                AccessError::FleetSubnetRootRequired,
                codes::AUTHORITY_UNAVAILABLE,
            ),
            (AccessError::ParentRequired, codes::AUTHORITY_UNAVAILABLE),
            (
                AccessError::NegatedPredicateMatched,
                codes::CONFIGURATION_INVALID_STATE,
            ),
            (
                AccessError::RequiredScopeMissing,
                codes::CONFIGURATION_UNAVAILABLE,
            ),
            (
                AccessError::RoleAttestationMalformed,
                codes::SECURITY_INVALID_STATE,
            ),
            (
                AccessError::RoleAttestationSubjectMismatch,
                codes::AUTHORITY_CONFLICT,
            ),
            (
                AccessError::RootOrActiveComponentRequired,
                codes::AUTHORITY_UNAVAILABLE,
            ),
            (AccessError::RootRequired, codes::AUTHORITY_UNAVAILABLE),
            (AccessError::SelfRequired, codes::AUTHORITY_UNAVAILABLE),
            (
                AccessError::ServiceAuthorityRequired,
                codes::AUTHORITY_UNAVAILABLE,
            ),
            (
                AccessError::WhitelistRequired,
                codes::CONFIGURATION_UNAVAILABLE,
            ),
        ];

        for (error, expected) in public {
            let actual = error.diagnostic_codes().expect("direct access identity");
            assert_eq!(actual.exact, expected);
            assert_eq!(actual.public, expected);
        }
    }

    #[test]
    fn projected_access_rejections_use_the_approved_registered_identities() {
        let projected = [
            (
                AccessError::ExpressionRuleRequired,
                codes::CONFIGURATION_UNAVAILABLE,
                codes::CONFIGURATION_INVALID,
            ),
            (
                AccessError::ServiceGuardInvalid,
                codes::AUTHORITY_INVALID,
                codes::CONFIGURATION_INVALID,
            ),
            (
                AccessError::DelegatedTokenMaxTtlOverflow,
                codes::TIME_CAPACITY,
                codes::CONFIGURATION_INVALID,
            ),
        ];

        for (error, expected_exact, expected_public) in projected {
            let actual = error.diagnostic_codes().expect("projected access identity");
            assert_eq!(actual.exact, expected_exact);
            assert_eq!(actual.public, expected_public);
        }
    }
}
