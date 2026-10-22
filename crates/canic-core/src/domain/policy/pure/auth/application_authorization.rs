//! Module: domain::policy::pure::auth::application_authorization
//!
//! Responsibility: decide Canic protected-authority transitions and define boundary outcomes.
//! Does not own: caller/time acquisition, storage, proof cryptography, serialization, or mutation.
//! Boundary: access and workflow supply model values; policy returns value-only decisions.

use crate::domain::value::Principal;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
use crate::model::auth::application_authorization::LocalApplicationAuthorityBinding;

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

/// Successful issuer-scoped application identity and its strict session expiry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthorizedApplicationSubject {
    pub issuer: Principal,
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
    MissingScope,
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
}
