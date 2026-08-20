//! Module: access::auth
//!
//! Responsibility: enforce auth predicates and expose the synchronous local application decision.
//! Does not own: endpoint response mapping, operation replay safety, or storage schema.
//! Boundary: access expressions call auth predicates before endpoint workflow execution.

mod attestation;
mod identity;
#[cfg(feature = "internal-test-fixtures")]
mod measurement;
mod predicates;
mod token;

use crate::{access::AccessError, cdk::types::Principal};
use std::fmt;

pub use crate::{
    domain::policy::pure::auth::application_authorization::{
        AuthorizedApplicationSubject, LocalApplicationAuthorizationDecision,
        LocalApplicationAuthorizationDenial,
    },
    model::auth::application_authorization::{
        ApplicationScope, ApplicationScopeError, ApplicationScopeRef, CanonicalApplicationScopes,
    },
};
use crate::{
    domain::policy::pure::auth::application_authorization::{
        LocalApplicationAuthorizationPolicyInput,
        authorize_local_application as authorize_local_application_policy,
    },
    ops::{auth::AuthOps, ic::IcOps, storage::auth::AuthStateOps},
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

/// Authorize one local application call without parsing payloads or mutating state.
#[must_use]
pub fn authorize_local_application(
    request: LocalApplicationAuthorizationRequest<'_>,
) -> LocalApplicationAuthorizationDecision {
    let actual_caller = IcOps::msg_caller();
    let now_ns = IcOps::now_nanos();
    if actual_caller == Principal::anonymous() || actual_caller != request.observed_transport_caller
    {
        return authorize_with_values(request, actual_caller, now_ns, false, None, None, false);
    }

    let authority = match AuthOps::local_application_authorization_authority() {
        Ok(Some(authority)) => authority,
        Ok(None) => {
            return authorize_with_values(request, actual_caller, now_ns, false, None, None, false);
        }
        Err(_) => {
            return authorize_with_values(request, actual_caller, now_ns, true, None, None, false);
        }
    };
    let Ok(session) = AuthStateOps::application_session(actual_caller) else {
        return authorize_with_values(request, actual_caller, now_ns, true, None, None, false);
    };
    let subject_admissible = session.as_ref().is_some_and(|session| {
        validate_application_subject(session.authenticated_subject()).is_ok()
    });
    authorize_with_values(
        request,
        actual_caller,
        now_ns,
        true,
        Some(&authority.snapshot),
        session.as_ref(),
        subject_admissible,
    )
}

fn authorize_with_values(
    request: LocalApplicationAuthorizationRequest<'_>,
    actual_caller: Principal,
    now_ns: u64,
    capability_enabled: bool,
    authority: Option<
        &crate::model::auth::application_authorization::LocalApplicationAuthoritySnapshot,
    >,
    session: Option<&crate::model::auth::application_authorization::LocalApplicationSession>,
    subject_admissible: bool,
) -> LocalApplicationAuthorizationDecision {
    authorize_local_application_policy(LocalApplicationAuthorizationPolicyInput {
        actual_caller,
        observed_caller: request.observed_transport_caller,
        now_ns,
        capability_enabled,
        authority,
        session,
        subject_admissible,
        required_scope: request.required_scope,
    })
}

///
/// ApplicationSubjectRejection
///
/// Reason an application subject cannot be accepted as a user identity.
/// Owned by access auth and used to reject infrastructure principals.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationSubjectRejection {
    Anonymous,
    ManagementCanister,
    LocalCanister,
    RootCanister,
    ParentCanister,
    SubnetCanister,
    FleetSubnetRootCanister,
    DirectChildCanister,
}

impl fmt::Display for ApplicationSubjectRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::Anonymous => "anonymous principals are not allowed",
            Self::ManagementCanister => "management canister principal is not allowed",
            Self::LocalCanister => "current canister principal is not allowed",
            Self::RootCanister => "root canister principal is not allowed",
            Self::ParentCanister => "parent canister principal is not allowed",
            Self::SubnetCanister => "subnet principal is not allowed",
            Self::FleetSubnetRootCanister => "Fleet Subnet Root principal is not allowed",
            Self::DirectChildCanister => "direct child canister principal is not allowed",
        };
        f.write_str(reason)
    }
}

/// validate_application_subject
///
/// Reject obvious canister and infrastructure identities for local application sessions.
pub fn validate_application_subject(subject: Principal) -> Result<(), ApplicationSubjectRejection> {
    identity::validate_application_subject(subject)
}

pub(crate) fn delegated_token_verified(
    authenticated_subject: Principal,
    required_scope: Option<&str>,
) -> Result<Principal, AccessError> {
    token::delegated_token_verified(authenticated_subject, required_scope)
}

// -----------------------------------------------------------------------------
// Caller & topology predicates
// -----------------------------------------------------------------------------

/// Require that the caller controls the current canister.
/// Allows controller-only maintenance calls.
pub async fn is_controller(caller: Principal) -> Result<(), AccessError> {
    predicates::is_controller(caller).await
}

/// Require that the caller appears in the configured whitelist.
/// Missing whitelist configuration fails closed.
pub async fn is_whitelisted(caller: Principal) -> Result<(), AccessError> {
    predicates::is_whitelisted(caller).await
}

/// Require that the caller is a direct child of the current canister.
pub async fn is_child(caller: Principal) -> Result<(), AccessError> {
    predicates::is_child(caller).await
}

/// Require that the caller is the configured parent canister.
pub async fn is_parent(caller: Principal) -> Result<(), AccessError> {
    predicates::is_parent(caller).await
}

/// Require that the caller equals the configured root canister.
pub async fn is_root(caller: Principal) -> Result<(), AccessError> {
    predicates::is_root(caller).await
}

/// Require that the caller is the currently executing canister.
pub async fn is_same_canister(caller: Principal) -> Result<(), AccessError> {
    predicates::is_same_canister(caller).await
}

/// Require a root-signed caller attestation bound to this canister's live Subnet.
pub async fn is_attested_local_subnet(caller: Principal) -> Result<(), AccessError> {
    attestation::is_attested_local_subnet(caller).await
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
    use crate::{
        ids::CanisterRole,
        model::auth::application_authorization::{
            ApplicationScope, CanonicalApplicationScopes, LocalApplicationAuthoritySnapshot,
            LocalApplicationSession,
        },
        test::{seams, support::fleet_key},
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn validate_application_subject_rejects_anonymous() {
        let _guard = seams::lock();
        let err = validate_application_subject(Principal::anonymous())
            .expect_err("anonymous must be rejected");
        assert_eq!(err, ApplicationSubjectRejection::Anonymous);
    }

    #[test]
    fn validate_application_subject_rejects_management_canister() {
        let _guard = seams::lock();
        let err = validate_application_subject(Principal::management_canister())
            .expect_err("management canister must be rejected");
        assert_eq!(err, ApplicationSubjectRejection::ManagementCanister);
    }

    #[test]
    fn validate_application_subject_rejects_direct_child() {
        let _guard = seams::lock();
        let child = p(31);
        crate::ops::storage::children::CanisterChildrenOps::import_direct_children(
            p(30),
            vec![(child, CanisterRole::new("session_subject_child"))],
        );

        let err = validate_application_subject(child)
            .expect_err("direct child canister must be rejected");
        assert_eq!(err, ApplicationSubjectRejection::DirectChildCanister);

        crate::ops::storage::children::CanisterChildrenOps::import_direct_children(p(30), vec![]);
    }

    #[test]
    fn synchronous_local_application_facade_values_preserve_closed_policy() {
        let caller = p(7);
        let fleet = fleet_key(3);
        let role = CanisterRole::new("component");
        let authority = LocalApplicationAuthoritySnapshot::new(fleet, role.clone(), 4);
        let session = LocalApplicationSession::new(
            caller,
            caller,
            p(8),
            fleet,
            role,
            CanonicalApplicationScopes::for_session(vec![
                ApplicationScope::parse("app:read").unwrap(),
            ])
            .unwrap(),
            4,
            10,
            100,
            [1; 32],
            [2; 32],
        )
        .unwrap();
        let request = LocalApplicationAuthorizationRequest {
            observed_transport_caller: caller,
            required_scope: ApplicationScopeRef::from_static("app:read"),
        };

        assert_eq!(
            authorize_with_values(
                request,
                caller,
                20,
                true,
                Some(&authority),
                Some(&session),
                true,
            ),
            LocalApplicationAuthorizationDecision::Allow(AuthorizedApplicationSubject {
                subject: caller,
                expires_at_ns: 100,
            })
        );
        assert_eq!(
            authorize_with_values(request, p(9), 20, false, None, None, false),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::CallerMismatch,
            )
        );
    }

    #[test]
    fn synchronous_local_application_facade_is_one_bounded_read_path() {
        let source = include_str!("mod.rs");
        let start = source
            .find("pub fn authorize_local_application(")
            .expect("public local application facade");
        let end = source[start..]
            .find("fn authorize_with_values(")
            .map_or(source.len(), |offset| start + offset);
        let body = &source[start..end];

        assert_eq!(body.matches("IcOps::msg_caller()").count(), 1);
        assert_eq!(body.matches("IcOps::now_nanos()").count(), 1);
        assert_eq!(
            body.matches("AuthOps::local_application_authorization_authority()")
                .count(),
            1
        );
        assert_eq!(
            body.matches("AuthStateOps::application_session(actual_caller)")
                .count(),
            1
        );
        for forbidden in [
            ".await", "spawn(", "timer", "cleanup", "record_", "log!", "set_", "clear_",
        ] {
            assert!(
                !body.contains(forbidden),
                "authorization facade contains forbidden operation {forbidden}"
            );
        }
    }
}
