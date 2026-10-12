//! Module: ops::auth::application_authorization
//!
//! Responsibility: project protected configuration and managed identity into one local authority.
//! Does not own: caller/time acquisition, session lookup, authorization policy, or persistence.
//! Boundary: endpoint and access adapters consume the same current authority projection.

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
    #[cfg_attr(
        not(feature = "auth-local-application-authorization"),
        expect(
            dead_code,
            reason = "the scope ceiling is consumed by the enabled SDK session engine"
        )
    )]
    pub allowed_token_scopes: Vec<String>,
    pub config: LocalApplicationAuthorizationConfig,
    pub snapshot: LocalApplicationAuthoritySnapshot,
}

impl AuthOps {
    /// Project a submitted token into the shared engine's current contract.
    #[cfg(feature = "auth-local-application-authorization")]
    pub(crate) fn application_session_token(
        token: &crate::dto::auth::DelegatedToken,
    ) -> Result<ic_auth_protocol_types::DelegatedToken, InternalError> {
        super::delegated::protocol::token(token).map_err(|_| InternalError::invalid_input())
    }

    /// Convert the canonical narrowing request without selecting lease policy.
    #[cfg(feature = "auth-local-application-authorization")]
    pub(crate) fn shared_application_session_request(
        scopes: &CanonicalApplicationScopes,
        ttl: Option<u64>,
    ) -> Result<ic_auth::session_request::SessionRequest, InternalError> {
        ic_auth::session_request::SessionRequest::new(
            scopes
                .as_slice()
                .iter()
                .map(|scope| scope.as_str().to_owned())
                .collect(),
            ttl,
        )
        .map_err(|_| InternalError::invalid_input())
    }

    /// Bind admission to current host generation and configured lease limits.
    #[cfg(feature = "auth-local-application-authorization")]
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
            max_proof_ttl_ns:
                crate::model::auth::application_authorization::MAX_APPLICATION_PROOF_LIFETIME_NS,
        }
    }

    /// Preserve typed endpoint diagnostics while delegating session decisions.
    #[cfg(feature = "auth-local-application-authorization")]
    pub(crate) fn map_application_session_error(
        error: ic_auth::session::SessionError,
    ) -> InternalError {
        use crate::diagnostics::codes;
        use ic_auth::session::SessionError as E;
        match error {
            E::Token(cause) => super::token::verification::map_shared_verification_error(cause),
            E::ReplayConflict => InternalError::public(codes::STATE_CONFLICT),
            E::Capacity | E::ByteCapacity => InternalError::public(codes::CAPACITY_LIMIT),
            E::GenerationExhausted => InternalError::public(codes::VERSION_CAPACITY),
            E::Disabled => InternalError::public(codes::CONFIGURATION_INACTIVE),
            E::StaleAuthority | E::ScopeRejected => {
                InternalError::public(codes::AUTHORITY_CONFLICT)
            }
            E::ProofIneligible | E::MissingOrExpired => {
                InternalError::public(codes::SECURITY_INVALID)
            }
            E::StorageFailure => InternalError::unavailable(),
            E::Canonical(_) | E::InvalidPolicyOrRecord => InternalError::invalid_input(),
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
            allowed_token_scopes: canister.auth.allowed_token_scopes,
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
