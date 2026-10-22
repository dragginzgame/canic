//! Module: ops::auth::application_authorization
//!
//! Responsibility: project protected configuration and managed identity into one local authority.
//! Does not own: caller/time acquisition, session lookup, authorization policy, or persistence.
//! Boundary: endpoint and access adapters consume the same current authority projection.

#![cfg(any(test, feature = "auth-local-application-authorization"))]

use super::{AuthOps, AuthValidationError};
#[cfg(any(test, feature = "auth-local-application-authorization"))]
use crate::model::auth::application_authorization::{
    ApplicationScope, CanonicalApplicationScopes, LocalApplicationAuthorityBinding,
};
use crate::{
    InternalError,
    config::schema::LocalApplicationAuthorizationConfig,
    model::auth::application_authorization::LocalApplicationAuthoritySnapshot,
    ops::{config::ConfigOps, runtime::env::EnvOps},
};
use canic_contracts::ids::{CanisterRole, FleetKey, ManagedCanisterBinding};

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
        let Some(config) = canister.auth.local_application_authorization else {
            return Ok(None);
        };
        let binding = EnvOps::managed_binding()?;
        let (fleet, role) = managed_authority(&binding);
        Ok(Some(LocalApplicationAuthorizationAuthority {
            config,
            snapshot: LocalApplicationAuthoritySnapshot::new(fleet, role),
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
        let root = EnvOps::root_pid()?;
        if verifier.root_canister_id != root {
            return Err(AuthValidationError::InvalidRootAuthority {
                expected: root,
                found: verifier.root_canister_id,
            }
            .into());
        }
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
    use crate::{
        cdk::types::Principal,
        config::{Config, schema::CanisterKind},
        diagnostics::codes,
        domain::auth::IC_ROOT_PUBLIC_KEY_RAW_LENGTH,
        ops::storage::auth::{
            LocalApplicationAuthorizationStateOps,
            application_sessions::ApplicationSessionTestStateGuard,
        },
        storage::stable::env::Env,
        test::config::install_model_for_role,
        test::{config::ConfigTestBuilder, seams, support::managed_component_binding},
        workflow::runtime::auth::RuntimeAuthWorkflow,
    };

    #[test]
    fn local_application_authority_uses_its_own_policy_and_managed_identity() {
        let _guard = seams::lock();
        let mut canister = ConfigTestBuilder::canister_config(CanisterKind::Service);
        canister.auth.local_application_authorization = Some(LocalApplicationAuthorizationConfig {
            allowed_scopes: vec!["app:read".to_owned()],
            default_session_ttl_secs: 60,
            maximum_session_ttl_secs: 120,
        });
        let _config = ConfigTestBuilder::new()
            .with_default_canister("app", canister)
            .install();
        let original = Env::export();
        let mut current = original.clone();
        let binding = managed_component_binding();
        let (fleet, role) = managed_authority(&binding);
        current.record.canister_role = Some(role.clone());
        current.record.component_spec = Some("default".parse().unwrap());
        current.record.managed_binding = Some(binding);
        Env::import(current);
        let result = AuthOps::local_application_authorization_authority();
        Env::import(original);
        let authority = result.unwrap().expect("configured local authority");

        assert_eq!(authority.snapshot.fleet(), fleet);
        assert_eq!(authority.snapshot.role(), &role);
        assert_eq!(authority.config.allowed_scopes, ["app:read"]);
        assert_eq!(authority.config.maximum_session_ttl_secs, 120);
    }

    #[test]
    fn local_application_reconciliation_requires_the_installed_root() {
        let _guard = seams::lock();
        let _state = ApplicationSessionTestStateGuard::empty();
        let root = Principal::from_slice(&[9; 29]);
        let mut config = Config::parse_toml(include_str!(
            "../../../../../canisters/audit/local_authorization_probe/canic.toml"
        ))
        .unwrap();
        config.auth.delegated_tokens.root_canister_id = Some(root.to_text());
        config.auth.delegated_tokens.ic_root_public_key_raw_hex =
            Some("07".repeat(IC_ROOT_PUBLIC_KEY_RAW_LENGTH));
        let _config = install_model_for_role(config, "app");
        let original = Env::export();
        let mut current = original.clone();
        current.record.canister_role = Some("app".into());
        current.record.component_spec = Some("default".parse().unwrap());
        current.record.managed_binding = Some(managed_component_binding());
        current.record.root_pid = Some(root);
        Env::import(current.clone());
        let initialized = RuntimeAuthWorkflow::reconcile_local_application_authority();
        let retained = LocalApplicationAuthorizationStateOps::application_authority_binding();
        let generation = LocalApplicationAuthorizationStateOps::application_authority_generation();

        current.record.root_pid = Some(Principal::from_slice(&[8; 29]));
        Env::import(current.clone());
        let mismatch = RuntimeAuthWorkflow::reconcile_local_application_authority();
        let after_mismatch = LocalApplicationAuthorizationStateOps::application_authority_binding();
        let generation_after_mismatch =
            LocalApplicationAuthorizationStateOps::application_authority_generation();
        current.record.root_pid = None;
        Env::import(current);
        let missing = RuntimeAuthWorkflow::reconcile_local_application_authority();
        Env::import(original);

        assert!(initialized.is_ok());
        assert!(retained.as_ref().unwrap().is_some());
        assert_eq!(mismatch.unwrap_err().code(), codes::AUTHORITY_CONFLICT);
        assert_eq!(
            missing.unwrap_err().code(),
            codes::CONFIGURATION_UNAVAILABLE
        );
        assert_eq!(after_mismatch, retained);
        assert_eq!(generation_after_mismatch, generation);
        assert_eq!(
            LocalApplicationAuthorizationStateOps::application_authority_binding(),
            retained
        );
        assert_eq!(
            LocalApplicationAuthorizationStateOps::application_authority_generation(),
            generation
        );
    }
}
