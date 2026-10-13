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

#[cfg(feature = "auth-local-application-authorization")]
use crate::ops::storage::auth::LocalApplicationAuthorizationStateOps;
use crate::{
    access::AccessError,
    cdk::types::Principal,
    domain::policy::pure::auth::application_authorization::{
        LocalApplicationAuthorizationPolicyInput,
        authorize_local_application as authorize_local_application_policy,
    },
    ops::{auth::AuthOps, ic::IcOps},
};
use std::fmt;

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
    #[cfg(feature = "auth-local-application-authorization")]
    {
        use ic_auth::session::SessionError;
        let subject_admissible = validate_application_subject(actual_caller).is_ok();
        // SDK live checks precede the host's subject/scope denial classification.
        let result =
            crate::workflow::auth::application_sessions::ApplicationSessionWorkflow::authorize(
                actual_caller,
                Some(request.required_scope.as_str()),
                now_ns,
                &authority,
                true,
            );
        let reason = match result {
            Ok(session) if subject_admissible => {
                return LocalApplicationAuthorizationDecision::Allow(
                    AuthorizedApplicationSubject {
                        subject: session.subject(),
                        expires_at_ns: session.expires_at_ns(),
                    },
                );
            }
            Ok(_) => LocalApplicationAuthorizationDenial::InadmissibleSubject,
            Err(SessionError::MissingOrExpired) => {
                match LocalApplicationAuthorizationStateOps::has_application_session(actual_caller)
                {
                    Ok(false) => LocalApplicationAuthorizationDenial::MissingSession,
                    Ok(true) => LocalApplicationAuthorizationDenial::Expired,
                    Err(_) => LocalApplicationAuthorizationDenial::AuthorityUnavailable,
                }
            }
            Err(SessionError::StaleAuthority) => {
                LocalApplicationAuthorizationDenial::StaleAuthority
            }
            Err(SessionError::ScopeRejected) if !subject_admissible => {
                LocalApplicationAuthorizationDenial::InadmissibleSubject
            }
            Err(SessionError::ScopeRejected) => LocalApplicationAuthorizationDenial::MissingScope,
            Err(SessionError::Disabled) => LocalApplicationAuthorizationDenial::Disabled,
            Err(_) => LocalApplicationAuthorizationDenial::AuthorityUnavailable,
        };
        LocalApplicationAuthorizationDecision::Deny(reason)
    }
    #[cfg(not(feature = "auth-local-application-authorization"))]
    {
        let _ = authority;
        LocalApplicationAuthorizationDecision::Deny(LocalApplicationAuthorizationDenial::Disabled)
    }
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

/// Require a root-signed caller attestation bound to this canister's live Subnet.
pub fn is_attested_local_subnet(caller: Principal) -> Result<(), AccessError> {
    attestation::is_attested_local_subnet(caller)
}

pub(crate) fn verify_decoded_token(
    token: &crate::dto::auth::DelegatedToken,
    caller: Principal,
    required_scope: Option<&str>,
) -> Result<Principal, AccessError> {
    token::verify_token(token, caller, IcOps::now_nanos(), required_scope)
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
    use crate::{
        model::auth::application_authorization::{
            ApplicationScope, CanonicalApplicationScopes, LocalApplicationAuthoritySnapshot,
            LocalApplicationSession,
        },
        test::{seams, support::fleet_key},
    };
    use canic_contracts::ids::CanisterRole;

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
        crate::ops::storage::children::CanisterChildrenOps::import_topology_children(
            p(30),
            vec![(child, CanisterRole::new("session_subject_child"))],
        );

        let err = validate_application_subject(child)
            .expect_err("direct child canister must be rejected");
        assert_eq!(err, ApplicationSubjectRejection::DirectChildCanister);

        crate::ops::storage::children::CanisterChildrenOps::import_topology_children(p(30), vec![]);
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
