//! Module: access::auth
//!
//! Responsibility: enforce auth predicates and expose the synchronous local application decision.
//! Does not own: endpoint response mapping, operation replay safety, or storage schema.
//! Boundary: access expressions call auth predicates before endpoint workflow execution.

mod attestation;
#[cfg(feature = "internal-test-fixtures")]
mod measurement;
mod predicates;

use crate::{access::AccessError, cdk::types::Principal, ops::ic::IcOps};

pub use crate::{
    domain::policy::pure::auth::application_authorization::AuthorizedApplicationSubject,
    domain::policy::pure::auth::application_authorization::LocalApplicationAuthorizationDecision,
    domain::policy::pure::auth::application_authorization::LocalApplicationAuthorizationDenial,
    model::auth::application_authorization::ApplicationScope,
    model::auth::application_authorization::ApplicationScopeError,
    model::auth::application_authorization::ApplicationScopeRef,
    model::auth::application_authorization::CanonicalApplicationScopes,
};

#[cfg(feature = "internal-test-fixtures")]
#[doc(hidden)]
pub use measurement::measure_local_application_authorization_denial;

/// Public synchronous request for one caller-bound local application decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalApplicationAuthorizationRequest<'a> {
    pub observed_transport_caller: Principal,
    pub required_scope: ApplicationScopeRef<'a>,
}

/// Return a closed application admission decision until portable Auth is integrated.
/// Caller mismatches retain their typed refusal; no session can grant access.
#[must_use]
pub fn authorize_local_application(
    request: LocalApplicationAuthorizationRequest<'_>,
) -> LocalApplicationAuthorizationDecision {
    let actual_caller = IcOps::msg_caller();

    authorize_with_values(request, actual_caller)
}

fn authorize_with_values(
    request: LocalApplicationAuthorizationRequest<'_>,
    actual_caller: Principal,
) -> LocalApplicationAuthorizationDecision {
    let denial = if actual_caller == Principal::anonymous() {
        LocalApplicationAuthorizationDenial::Anonymous
    } else if actual_caller != request.observed_transport_caller {
        LocalApplicationAuthorizationDenial::CallerMismatch
    } else {
        LocalApplicationAuthorizationDenial::AuthorityUnavailable
    };
    LocalApplicationAuthorizationDecision::Deny(denial)
}

// -----------------------------------------------------------------------------
// Caller & topology predicates
// -----------------------------------------------------------------------------

/// Require that the caller controls the current canister.
/// Allows controller-only maintenance calls.
pub fn is_controller(caller: Principal) -> Result<(), AccessError> {
    predicates::is_controller(caller)
}

/// Require the actual caller to be a controller or the exact stable Root.
pub fn is_controller_or_root(caller: Principal) -> Result<(), AccessError> {
    predicates::is_controller_or_root(caller)
}

/// Require that the caller appears in the exact open Fleet projection.
/// Missing, invalid or fenced stable authority fails closed.
pub fn is_fleet_admitted(caller: Principal) -> Result<(), AccessError> {
    predicates::require_fleet_admission(caller).map(|_caller| ())
}

/// Return the observed transport caller only when the exact local Fleet projection admits it.
/// Missing, invalid or fenced stable authority fails closed.
pub fn require_fleet_admitted_caller() -> Result<Principal, AccessError> {
    let caller = IcOps::msg_caller();
    predicates::require_fleet_admission(caller)
}

/// Require that the caller is a direct child of the current canister.
pub fn is_child(caller: Principal) -> Result<(), AccessError> {
    predicates::is_child(caller)
}

/// Require that the caller is the configured parent canister.
pub fn is_parent(caller: Principal) -> Result<(), AccessError> {
    predicates::is_parent(caller)
}

/// Require that the caller equals the configured root canister.
pub fn is_root(caller: Principal) -> Result<(), AccessError> {
    predicates::is_root(caller)
}

/// Require that the caller is the currently executing canister.
pub fn is_same_canister(caller: Principal) -> Result<(), AccessError> {
    predicates::is_same_canister(caller)
}

pub(crate) fn verify_decoded_attestation(
    attestation: &crate::dto::auth::SignedRoleAttestation,
    caller: Principal,
) -> Result<(), AccessError> {
    attestation::verify_attestation(attestation, caller)
}

const fn dependency_unavailable(error: crate::InternalError) -> AccessError {
    AccessError::Internal(error)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn synchronous_local_application_facade_values_preserve_closed_policy() {
        let request = LocalApplicationAuthorizationRequest {
            observed_transport_caller: p(7),
            required_scope: ApplicationScopeRef::from_static("app:read"),
        };
        for (caller, denial) in [
            (
                Principal::anonymous(),
                LocalApplicationAuthorizationDenial::Anonymous,
            ),
            (p(9), LocalApplicationAuthorizationDenial::CallerMismatch),
            (
                p(7),
                LocalApplicationAuthorizationDenial::AuthorityUnavailable,
            ),
        ] {
            assert_eq!(
                authorize_with_values(request, caller),
                LocalApplicationAuthorizationDecision::Deny(denial)
            );
        }
    }

    #[test]
    fn synchronous_fleet_admission_facade_owns_caller_acquisition_only() {
        let source = include_str!("mod.rs");
        let start = source
            .find("pub fn require_fleet_admitted_caller(")
            .expect("public Fleet-admission caller facade");
        let end = source[start..]
            .find("/// Require that the caller is a direct child")
            .map_or(source.len(), |offset| start + offset);
        let body = &source[start..end];

        assert_eq!(body.matches("IcOps::msg_caller()").count(), 1);
        assert_eq!(
            body.matches("predicates::require_fleet_admission(caller)")
                .count(),
            1
        );
        for forbidden in [
            ".await", "spawn(", "timer", "cleanup", "record_", "log!", "set_", "clear_",
        ] {
            assert!(
                !body.contains(forbidden),
                "Fleet-admission facade contains forbidden operation {forbidden}"
            );
        }
    }
}
