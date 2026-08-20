//! Module: ops::auth::application_authorization
//!
//! Responsibility: project protected configuration and managed identity into one local authority.
//! Does not own: caller/time acquisition, session lookup, authorization policy, or persistence.
//! Boundary: endpoint and access adapters consume the same current authority projection.

use super::AuthOps;
use crate::{
    InternalError,
    config::schema::LocalApplicationAuthorizationConfig,
    ids::{CanisterRole, FleetKey, ManagedCanisterBinding},
    model::auth::application_authorization::{
        ApplicationScope, CanonicalApplicationScopes, LocalApplicationAuthorityBinding,
        LocalApplicationAuthoritySnapshot,
    },
    ops::{config::ConfigOps, runtime::env::EnvOps, storage::auth::AuthStateOps},
};

/// Protected configuration and exact current identity for local application authorization.
pub struct LocalApplicationAuthorizationAuthority {
    pub config: LocalApplicationAuthorizationConfig,
    pub snapshot: LocalApplicationAuthoritySnapshot,
}

impl AuthOps {
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
                AuthStateOps::application_authority_generation(),
            ),
        }))
    }

    /// Project the current protected binding used for local generation transitions.
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
