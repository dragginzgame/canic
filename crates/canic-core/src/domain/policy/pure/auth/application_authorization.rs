//! Module: domain::policy::pure::auth::application_authorization
//!
//! Responsibility: make closed local-application authorization and admission decisions.
//! Does not own: caller/time acquisition, storage, proof cryptography, serialization, or mutation.
//! Boundary: access and workflow supply model values; policy returns value-only decisions.

#[cfg(any(test, feature = "auth-local-application-authorization"))]
use crate::model::auth::application_authorization::LocalApplicationAuthorityBinding;
use crate::{
    domain::value::Principal,
    model::auth::application_authorization::{
        ApplicationScopeRef, LocalApplicationAuthoritySnapshot, LocalApplicationSession,
        MAX_LOCAL_APPLICATION_SESSION_TTL_NS,
    },
};

/// Stable-binding action required by one locally activated protected-policy change.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationAuthorityBindingTransition {
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    AdvanceGeneration,
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    Initialize,
    Unchanged,
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    UpdateWithoutGeneration,
}

/// Decide whether one protected local policy transition invalidates retained sessions.
#[must_use]
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub fn decide_application_authority_binding_transition(
    previous: Option<&LocalApplicationAuthorityBinding>,
    current: &LocalApplicationAuthorityBinding,
) -> ApplicationAuthorityBindingTransition {
    let Some(previous) = previous else {
        return ApplicationAuthorityBindingTransition::Initialize;
    };
    if previous == current {
        return ApplicationAuthorityBindingTransition::Unchanged;
    }
    match (previous, current) {
        (
            LocalApplicationAuthorityBinding::Disabled,
            LocalApplicationAuthorityBinding::Enabled { .. },
        ) => ApplicationAuthorityBindingTransition::AdvanceGeneration,
        (
            LocalApplicationAuthorityBinding::Enabled { .. }
            | LocalApplicationAuthorityBinding::Disabled,
            LocalApplicationAuthorityBinding::Disabled,
        ) => ApplicationAuthorityBindingTransition::UpdateWithoutGeneration,
        (
            LocalApplicationAuthorityBinding::Enabled {
                fleet: previous_fleet,
                role: previous_role,
                verifier_root_canister_id: previous_root,
                minimum_accepted_registry_epoch: previous_registry_epoch,
                allowed_scopes: previous_scopes,
                maximum_session_ttl_secs: previous_maximum_ttl,
            },
            LocalApplicationAuthorityBinding::Enabled {
                fleet: current_fleet,
                role: current_role,
                verifier_root_canister_id: current_root,
                minimum_accepted_registry_epoch: current_registry_epoch,
                allowed_scopes: current_scopes,
                maximum_session_ttl_secs: current_maximum_ttl,
            },
        ) => {
            let scope_removed = previous_scopes
                .as_slice()
                .iter()
                .any(|scope| !current_scopes.contains(scope.as_scope_ref()));
            if previous_fleet != current_fleet
                || previous_role != current_role
                || previous_root != current_root
                || previous_registry_epoch != current_registry_epoch
                || scope_removed
                || current_maximum_ttl < previous_maximum_ttl
            {
                ApplicationAuthorityBindingTransition::AdvanceGeneration
            } else {
                ApplicationAuthorityBindingTransition::UpdateWithoutGeneration
            }
        }
    }
}

/// Pure authorization inputs acquired once by the synchronous access boundary.
pub struct LocalApplicationAuthorizationPolicyInput<'a> {
    pub actual_caller: Principal,
    pub observed_caller: Principal,
    pub now_ns: u64,
    pub capability_enabled: bool,
    pub authority: Option<&'a LocalApplicationAuthoritySnapshot>,
    pub session: Option<&'a LocalApplicationSession>,
    pub subject_admissible: bool,
    pub required_scope: ApplicationScopeRef<'a>,
}

/// Successful local application identity and its strict session expiry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthorizedApplicationSubject {
    pub subject: Principal,
    pub expires_at_ns: u64,
}

/// Closed synchronous local application authorization result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalApplicationAuthorizationDecision {
    Allow(AuthorizedApplicationSubject),
    Deny(LocalApplicationAuthorizationDenial),
}

/// Closed denial reasons in their normative evaluation precedence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalApplicationAuthorizationDenial {
    Anonymous,
    CallerMismatch,
    Disabled,
    AuthorityUnavailable,
    MissingSession,
    Expired,
    StaleAuthority,
    InadmissibleSubject,
    MissingScope,
}

/// Authorize one already-validated scope using only supplied values.
#[must_use]
pub fn authorize_local_application(
    input: LocalApplicationAuthorizationPolicyInput<'_>,
) -> LocalApplicationAuthorizationDecision {
    let denial = if input.actual_caller == Principal::anonymous() {
        Some(LocalApplicationAuthorizationDenial::Anonymous)
    } else if input.actual_caller != input.observed_caller {
        Some(LocalApplicationAuthorizationDenial::CallerMismatch)
    } else if !input.capability_enabled {
        Some(LocalApplicationAuthorizationDenial::Disabled)
    } else if input.authority.is_none() {
        Some(LocalApplicationAuthorizationDenial::AuthorityUnavailable)
    } else if input.session.is_none() {
        Some(LocalApplicationAuthorizationDenial::MissingSession)
    } else {
        None
    };
    if let Some(denial) = denial {
        return LocalApplicationAuthorizationDecision::Deny(denial);
    }

    let authority = input
        .authority
        .expect("checked protected authority presence");
    let session = input.session.expect("checked canonical session presence");
    if input.now_ns >= session.expires_at_ns() {
        return LocalApplicationAuthorizationDecision::Deny(
            LocalApplicationAuthorizationDenial::Expired,
        );
    }
    if session.transport_caller() != input.actual_caller
        || session.fleet() != authority.fleet()
        || session.role() != authority.role()
        || session.authority_generation() != authority.generation()
    {
        return LocalApplicationAuthorizationDecision::Deny(
            LocalApplicationAuthorizationDenial::StaleAuthority,
        );
    }
    if !input.subject_admissible {
        return LocalApplicationAuthorizationDecision::Deny(
            LocalApplicationAuthorizationDenial::InadmissibleSubject,
        );
    }
    if !session.scopes().contains(input.required_scope) {
        return LocalApplicationAuthorizationDecision::Deny(
            LocalApplicationAuthorizationDenial::MissingScope,
        );
    }
    LocalApplicationAuthorizationDecision::Allow(AuthorizedApplicationSubject {
        subject: session.authenticated_subject(),
        expires_at_ns: session.expires_at_ns(),
    })
}

/// Resolve one strict session expiry from protected and requested TTL values.
pub fn resolve_application_session_expiry(
    established_at_ns: u64,
    default_ttl_ns: u64,
    maximum_ttl_ns: u64,
    requested_ttl_ns: Option<u64>,
) -> Result<u64, ApplicationSessionTtlError> {
    if default_ttl_ns == 0 {
        return Err(ApplicationSessionTtlError::DefaultZero);
    }
    if maximum_ttl_ns == 0 {
        return Err(ApplicationSessionTtlError::MaximumZero);
    }
    if default_ttl_ns > maximum_ttl_ns {
        return Err(ApplicationSessionTtlError::DefaultExceedsMaximum);
    }
    if maximum_ttl_ns > MAX_LOCAL_APPLICATION_SESSION_TTL_NS {
        return Err(ApplicationSessionTtlError::MaximumExceedsHardLimit {
            maximum_ttl_ns,
            hard_limit_ns: MAX_LOCAL_APPLICATION_SESSION_TTL_NS,
        });
    }
    let selected_ttl_ns = requested_ttl_ns.unwrap_or(default_ttl_ns);
    if selected_ttl_ns == 0 {
        return Err(ApplicationSessionTtlError::RequestedZero);
    }
    if selected_ttl_ns > maximum_ttl_ns {
        return Err(ApplicationSessionTtlError::RequestedExceedsMaximum);
    }
    established_at_ns
        .checked_add(selected_ttl_ns)
        .ok_or(ApplicationSessionTtlError::ExpiryOverflow)
}

/// Typed protected/requested local session TTL failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationSessionTtlError {
    DefaultExceedsMaximum,
    DefaultZero,
    ExpiryOverflow,
    MaximumExceedsHardLimit {
        maximum_ttl_ns: u64,
        hard_limit_ns: u64,
    },
    MaximumZero,
    RequestedExceedsMaximum,
    RequestedZero,
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::auth::application_authorization::CanonicalApplicationScopes;
    use crate::{
        model::auth::application_authorization::ApplicationScope, test::support::fleet_key,
    };
    use canic_contracts::ids::CanisterRole;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn scopes(values: &[&str]) -> CanonicalApplicationScopes {
        CanonicalApplicationScopes::for_session(
            values
                .iter()
                .map(|value| ApplicationScope::parse(*value).unwrap())
                .collect(),
        )
        .unwrap()
    }

    fn session() -> LocalApplicationSession {
        LocalApplicationSession::new(
            p(1),
            p(1),
            p(2),
            fleet_key(3),
            CanisterRole::new("component"),
            scopes(&["app:read"]),
            4,
            10,
            100,
            [5; 32],
            [6; 32],
        )
        .unwrap()
    }

    fn authority() -> LocalApplicationAuthoritySnapshot {
        LocalApplicationAuthoritySnapshot::new(fleet_key(3), CanisterRole::new("component"), 4)
    }

    fn binding(
        fleet: u8,
        role: &'static str,
        root: u8,
        registry_epoch: u64,
        allowed_scopes: &[&str],
        maximum_ttl_secs: u64,
    ) -> LocalApplicationAuthorityBinding {
        LocalApplicationAuthorityBinding::enabled(
            fleet_key(fleet),
            CanisterRole::new(role),
            p(root),
            Some(registry_epoch),
            scopes(allowed_scopes),
            maximum_ttl_secs,
        )
    }

    fn authorization_input<'a>(
        authority: Option<&'a LocalApplicationAuthoritySnapshot>,
        session: Option<&'a LocalApplicationSession>,
    ) -> LocalApplicationAuthorizationPolicyInput<'a> {
        LocalApplicationAuthorizationPolicyInput {
            actual_caller: p(1),
            observed_caller: p(1),
            now_ns: 50,
            capability_enabled: true,
            authority,
            session,
            subject_admissible: true,
            required_scope: ApplicationScopeRef::from_static("app:read"),
        }
    }

    #[test]
    fn authorization_allows_exact_current_session_scope() {
        let authority = authority();
        let session = session();
        assert_eq!(
            authorize_local_application(authorization_input(Some(&authority), Some(&session))),
            LocalApplicationAuthorizationDecision::Allow(AuthorizedApplicationSubject {
                subject: p(1),
                expires_at_ns: 100,
            })
        );
    }

    #[test]
    fn authority_binding_transition_applies_the_frozen_generation_table() {
        let original = binding(1, "component", 9, 4, &["app:read"], 900);
        assert_eq!(
            decide_application_authority_binding_transition(None, &original),
            ApplicationAuthorityBindingTransition::Initialize
        );
        assert_eq!(
            decide_application_authority_binding_transition(Some(&original), &original),
            ApplicationAuthorityBindingTransition::Unchanged
        );

        for future_only in [
            binding(1, "component", 9, 4, &["app:read", "app:write"], 900),
            binding(1, "component", 9, 4, &["app:read"], 1_800),
        ] {
            assert_eq!(
                decide_application_authority_binding_transition(Some(&original), &future_only),
                ApplicationAuthorityBindingTransition::UpdateWithoutGeneration
            );
        }

        for invalidating in [
            binding(2, "component", 9, 4, &["app:read"], 900),
            binding(1, "other", 9, 4, &["app:read"], 900),
            binding(1, "component", 8, 4, &["app:read"], 900),
            binding(1, "component", 9, 5, &["app:read"], 900),
            binding(1, "component", 9, 4, &["app:other"], 900),
            binding(1, "component", 9, 4, &["app:read"], 899),
        ] {
            assert_eq!(
                decide_application_authority_binding_transition(Some(&original), &invalidating),
                ApplicationAuthorityBindingTransition::AdvanceGeneration
            );
        }

        assert_eq!(
            decide_application_authority_binding_transition(
                Some(&original),
                &LocalApplicationAuthorityBinding::Disabled,
            ),
            ApplicationAuthorityBindingTransition::UpdateWithoutGeneration
        );
        assert_eq!(
            decide_application_authority_binding_transition(
                Some(&LocalApplicationAuthorityBinding::Disabled),
                &original,
            ),
            ApplicationAuthorityBindingTransition::AdvanceGeneration
        );
    }

    #[test]
    fn authorization_denial_precedence_is_closed() {
        let authority = authority();
        let session = session();
        let mut input = authorization_input(Some(&authority), Some(&session));
        input.actual_caller = Principal::anonymous();
        input.observed_caller = p(9);
        input.capability_enabled = false;
        input.authority = None;
        input.session = None;
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::Anonymous
            )
        );

        let mut input = authorization_input(Some(&authority), Some(&session));
        input.observed_caller = p(9);
        input.capability_enabled = false;
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::CallerMismatch
            )
        );

        let mut input = authorization_input(None, None);
        input.capability_enabled = false;
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::Disabled
            )
        );
        assert_eq!(
            authorize_local_application(authorization_input(None, None)),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::AuthorityUnavailable
            )
        );
        assert_eq!(
            authorize_local_application(authorization_input(Some(&authority), None)),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::MissingSession
            )
        );
    }

    #[test]
    fn authorization_applies_expiry_staleness_admissibility_and_scope_order() {
        let authority = authority();
        let session = session();

        let mut input = authorization_input(Some(&authority), Some(&session));
        input.now_ns = session.expires_at_ns();
        input.subject_admissible = false;
        input.required_scope = ApplicationScopeRef::from_static("app:write");
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::Expired
            )
        );

        let stale_authority =
            LocalApplicationAuthoritySnapshot::new(fleet_key(3), CanisterRole::new("component"), 5);
        let mut input = authorization_input(Some(&stale_authority), Some(&session));
        input.subject_admissible = false;
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::StaleAuthority
            )
        );

        let mut input = authorization_input(Some(&authority), Some(&session));
        input.subject_admissible = false;
        input.required_scope = ApplicationScopeRef::from_static("app:write");
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::InadmissibleSubject
            )
        );

        let mut input = authorization_input(Some(&authority), Some(&session));
        input.required_scope = ApplicationScopeRef::from_static("app:write");
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::MissingScope
            )
        );
    }

    #[test]
    fn ttl_resolution_uses_independent_protected_session_clock() {
        assert_eq!(resolve_application_session_expiry(10, 20, 30, None), Ok(30));
        assert_eq!(
            resolve_application_session_expiry(10, 20, 30, Some(30)),
            Ok(40)
        );
        assert_eq!(
            resolve_application_session_expiry(10, 20, 30, Some(31)),
            Err(ApplicationSessionTtlError::RequestedExceedsMaximum)
        );
        assert_eq!(
            resolve_application_session_expiry(
                10,
                20,
                MAX_LOCAL_APPLICATION_SESSION_TTL_NS + 1,
                None
            ),
            Err(ApplicationSessionTtlError::MaximumExceedsHardLimit {
                maximum_ttl_ns: MAX_LOCAL_APPLICATION_SESSION_TTL_NS + 1,
                hard_limit_ns: MAX_LOCAL_APPLICATION_SESSION_TTL_NS,
            })
        );
    }
}
