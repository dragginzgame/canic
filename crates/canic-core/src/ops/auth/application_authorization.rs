//! Module: ops::auth::application_authorization
//!
//! Responsibility: project protected configuration and managed identity into one local authority.
//! Does not own: caller/time acquisition, session lookup, authorization policy, or persistence.
//! Boundary: endpoint and access adapters consume the same current authority projection.

#![cfg(any(test, feature = "auth-local-application-authorization"))]

use super::AuthOps;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
use crate::model::auth::application_authorization::{
    ApplicationScope, CanonicalApplicationScopes, LocalApplicationAuthorityBinding,
};
use crate::{
    InternalError,
    config::schema::LocalApplicationAuthorizationConfig,
    model::auth::application_authorization::LocalApplicationAuthoritySnapshot,
    ops::{
        config::ConfigOps, runtime::env::EnvOps,
        storage::auth::LocalApplicationAuthorizationStateOps,
    },
};
use canic_contracts::ids::{CanisterRole, FleetKey, ManagedCanisterBinding};

/// Protected configuration and exact current identity for local application authorization.

pub struct LocalApplicationAuthorizationAuthority {
    pub config: LocalApplicationAuthorizationConfig,
    pub snapshot: LocalApplicationAuthoritySnapshot,
}

impl AuthOps {
    /// Bind admission to current host generation and configured lease limits.
    #[cfg(test)]
    pub(crate) const fn shared_application_session_policy(
        authority: &LocalApplicationAuthorizationAuthority,
        subject_admissible: bool,
    ) -> ic_auth::session::SessionPolicy {
        ic_auth::session::SessionPolicy {
            generation: authority.snapshot.generation(),
            enabled: true,
            subject_admissible,
            default_ttl_secs: authority.config.default_session_ttl_secs,
            max_ttl_secs: authority.config.maximum_session_ttl_secs,
        }
    }

    /// Preserve typed endpoint diagnostics while delegating session decisions.
    #[cfg(test)]
    pub(crate) fn map_application_session_error(
        error: ic_auth::session::SessionError,
    ) -> InternalError {
        use crate::diagnostics::codes;
        use ic_auth::{
            portable::LoginRevocationError as R, portable_certified::PortableCertifiedError as C,
            portable_credential::PortableCredentialError as P,
            portable_signature::PortableSignatureError as S, session::SessionError as E,
        };
        match error {
            // Preserve resource and clock failures through either verifier envelope.
            E::Credential(P::Ed25519(cause)) => {
                Self::map_application_session_error(E::Signature(cause))
            }
            E::Credential(P::Certified(cause)) => {
                Self::map_application_session_error(E::Certified(cause))
            }
            E::Signature(S::Budget(cause)) | E::Certified(C::Budget(cause)) => {
                Self::map_application_session_error(cause.into())
            }
            E::ReplayConflict => InternalError::public(codes::STATE_CONFLICT),
            E::AdmissionThrottled | E::Capacity | E::ByteCapacity | E::Revocation(R::Capacity) => {
                InternalError::public(codes::CAPACITY_LIMIT)
            }
            E::AdmissionClockRollback | E::Revocation(R::Clock) => {
                InternalError::public(codes::TIME_INVALID)
            }
            E::GenerationExhausted => InternalError::public(codes::VERSION_CAPACITY),
            E::Disabled => InternalError::public(codes::CONFIGURATION_INACTIVE),
            E::StaleAuthority | E::ScopeRejected => {
                InternalError::public(codes::AUTHORITY_CONFLICT)
            }
            E::MissingOrExpired
            | E::Claims(_)
            | E::Signature(_)
            | E::Certified(_)
            | E::Credential(P::MechanismMismatch)
            | E::Revocation(R::InvalidCaller | R::Revoked) => {
                InternalError::public(codes::SECURITY_INVALID)
            }
            E::StorageFailure | E::Revocation(R::Storage | R::InvalidPolicy) => {
                InternalError::unavailable()
            }
            E::Canonical(_) | E::Encoding(_) | E::InvalidPolicyOrRecord => {
                InternalError::invalid_input()
            }
        }
    }

    /// Project the one protected local application authority, or `None` when disabled.
    pub(crate) fn local_application_authorization_authority()
    -> Result<Option<LocalApplicationAuthorizationAuthority>, InternalError> {
        let canister = ConfigOps::current_canister()?;
        if !canister.auth.delegated_token_verifier {
            return Ok(None);
        }
        let Some(config) = canister.auth.local_application_authorization else {
            return Ok(None);
        };
        let binding = EnvOps::managed_binding()?;
        let (fleet, role) = managed_authority(&binding);
        Ok(Some(LocalApplicationAuthorizationAuthority {
            config,
            snapshot: LocalApplicationAuthoritySnapshot::new(
                fleet,
                role,
                LocalApplicationAuthorizationStateOps::application_authority_generation(),
            ),
        }))
    }

    /// Project the current protected binding used for local generation transitions.
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    pub(crate) fn local_application_authority_binding()
    -> Result<LocalApplicationAuthorityBinding, InternalError> {
        let Some(authority) = Self::local_application_authorization_authority()? else {
            return Ok(LocalApplicationAuthorityBinding::Disabled);
        };
        let verifier = Self::auth_proof_verifier_config()?;
        let allowed_scopes = authority
            .config
            .allowed_scopes
            .iter()
            .map(|scope| ApplicationScope::parse(scope.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| InternalError::invariant())?;
        let allowed_scopes = CanonicalApplicationScopes::for_verified_grant(allowed_scopes)
            .map_err(|_| InternalError::invariant())?;
        Ok(LocalApplicationAuthorityBinding::enabled(
            authority.snapshot.fleet(),
            authority.snapshot.role().clone(),
            verifier.root_canister_id,
            verifier
                .chain_key_root
                .map(|chain_key| chain_key.policy.min_accepted_registry_epoch),
            allowed_scopes,
            authority.config.maximum_session_ttl_secs,
        ))
    }
}

fn managed_authority(binding: &ManagedCanisterBinding) -> (FleetKey, CanisterRole) {
    match binding {
        ManagedCanisterBinding::Component(component) => (
            component.authority.binding.fleet.fleet,
            component.role.clone(),
        ),
        ManagedCanisterBinding::ComponentChild(child) => (
            child.component.authority.binding.fleet.fleet,
            child.role.clone(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{diagnostics::codes, test::support::fleet_key};
    use ic_auth::{
        admission_budget::AdmissionBudgetError, portable::LoginRevocationError,
        portable_certified::PortableCertifiedError, portable_credential::PortableCredentialError,
        portable_signature::PortableSignatureError, session::SessionError,
    };

    #[test]
    fn verifier_envelopes_preserve_budget_and_clock_diagnostics() {
        for (cause, expected) in [
            (AdmissionBudgetError::Exhausted, codes::CAPACITY_LIMIT),
            (AdmissionBudgetError::ClockRollback, codes::TIME_INVALID),
        ] {
            let certified = SessionError::Credential(PortableCredentialError::Certified(
                PortableCertifiedError::Budget(cause),
            ));
            let signature = SessionError::Credential(PortableCredentialError::Ed25519(
                PortableSignatureError::Budget(cause),
            ));
            for error in [certified, signature] {
                assert_eq!(
                    AuthOps::map_application_session_error(error).code(),
                    expected
                );
            }
        }
        for (cause, expected) in [
            (LoginRevocationError::Revoked, codes::SECURITY_INVALID),
            (LoginRevocationError::Storage, codes::STATE_UNAVAILABLE),
            (LoginRevocationError::Clock, codes::TIME_INVALID),
            (LoginRevocationError::Capacity, codes::CAPACITY_LIMIT),
        ] {
            assert_eq!(
                AuthOps::map_application_session_error(cause.into()).code(),
                expected
            );
        }
    }

    #[test]
    fn session_policy_retains_protected_generation_limits_and_subject_refusal() {
        let authority = LocalApplicationAuthorizationAuthority {
            config: LocalApplicationAuthorizationConfig {
                allowed_scopes: vec!["read".to_owned()],
                default_session_ttl_secs: 30,
                maximum_session_ttl_secs: 90,
            },
            snapshot: LocalApplicationAuthoritySnapshot::new(
                fleet_key(1),
                CanisterRole::new("app_service"),
                7,
            ),
        };
        let policy = AuthOps::shared_application_session_policy(&authority, false);
        assert_eq!(policy.generation, 7);
        assert_eq!(policy.default_ttl_secs, 30);
        assert_eq!(policy.max_ttl_secs, 90);
        assert!(policy.enabled);
        assert!(!policy.subject_admissible);
    }
}
