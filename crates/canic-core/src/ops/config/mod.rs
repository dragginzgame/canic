//! Module: ops::config
//!
//! Responsibility: expose fallible configuration lookups for ops and workflows.
//! Does not own: config parsing, environment initialization, or endpoint DTOs.
//! Boundary: ops layer between runtime context and immutable configuration model.

use crate::{
    InternalError,
    config::{
        ComponentTopology, Config, ConfigError, ConfigModel, RoleRuntimeConfig,
        RuntimeCanisterConfig,
        schema::{
            CanisterConfig, ComponentSpecConfig, DelegatedTokenConfig, FleetInitMode, IndexConfig,
            LocalApplicationAuthorizationConfig, LogConfig, RoleAttestationConfig, ScalingConfig,
            implicit_root_canister_config, implicit_wasm_store_canister_config,
        },
    },
    dto::component_deployment::ProtectedComponentDeployment,
    ids::{CanisterRole, ComponentBinding, ComponentSpecId},
    model::cycles_funding::FundingLimits,
    ops::runtime::env::EnvOps,
    storage::stable::state::fleet::FleetMode,
};
use std::sync::Arc;
use thiserror::Error as ThisError;

///
/// ConfigOpsError
///
/// Typed failure surface for configuration lookup operations.
///

#[derive(Debug, ThisError)]
pub enum ConfigOpsError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error("Component Spec {0} not found in configuration")]
    ComponentSpecNotFound(String),

    #[error("canister {0} not defined in Component Spec {1}")]
    CanisterNotFound(String, String),

    #[error(
        "canister role {0} belongs to multiple Component Specs; an exact Component Spec binding is required"
    )]
    CanisterRoleAmbiguous(String),
}

impl From<ConfigOpsError> for InternalError {
    fn from(err: ConfigOpsError) -> Self {
        use crate::diagnostics::codes;

        match err {
            ConfigOpsError::Config(err) => err.into(),
            ConfigOpsError::ComponentSpecNotFound(_) | ConfigOpsError::CanisterNotFound(_, _) => {
                Self::public(codes::CONFIGURATION_UNAVAILABLE)
            }
            ConfigOpsError::CanisterRoleAmbiguous(_) => Self::public(codes::CONFIGURATION_CONFLICT),
        }
    }
}

/// Full configuration authority used only by the Root control plane.
pub struct RootConfigOps;

impl RootConfigOps {
    /// Export the full current configuration as TOML.
    /// Intended for diagnostics and tooling only.
    pub fn export_toml() -> Result<String, InternalError> {
        let toml = Config::to_toml()?;

        Ok(toml)
    }

    // ---------------------------------------------------------------------
    // Explicit / fallible lookups
    // ---------------------------------------------------------------------

    /// Fetch a Component Spec configuration by declared identity.
    pub(crate) fn try_get_component_spec(
        component_spec: &ComponentSpecId,
    ) -> Result<ComponentSpecConfig, InternalError> {
        let cfg = Config::get()?;

        cfg.get_component_spec(component_spec)
            .ok_or_else(|| ConfigOpsError::ComponentSpecNotFound(component_spec.to_string()).into())
    }

    /// Fetch a canister configuration within a specific Component Spec.
    pub(crate) fn try_get_canister(
        component_spec: &ComponentSpecId,
        canister_role: &CanisterRole,
    ) -> Result<CanisterConfig, InternalError> {
        let component_spec_cfg = Self::try_get_component_spec(component_spec)?;

        component_spec_cfg
            .get_canister(canister_role)
            .ok_or_else(|| {
                ConfigOpsError::CanisterNotFound(
                    canister_role.to_string(),
                    component_spec.to_string(),
                )
                .into()
            })
    }

    /// Compile the exact current Component Topology and its protected Spec hashes.
    pub fn component_topology() -> Result<ComponentTopology, InternalError> {
        Config::get()?
            .compile_component_topology()
            .map_err(ConfigError::from)
            .map_err(InternalError::from)
    }

    /// Validate one retained deployment context against the current compiled App authority.
    pub fn validate_protected_component_deployment(
        context: &ProtectedComponentDeployment,
        owning_component: &ComponentBinding,
    ) -> Result<(), InternalError> {
        Config::get()?
            .validate_protected_component_deployment(context, owning_component)
            .map_err(|_error| InternalError::invalid_input())
    }

    /// Resolve the exact configured package identity for one declared application role.
    pub fn role_package(canister_role: &CanisterRole) -> Result<String, InternalError> {
        let config = Config::get()?;
        config
            .roles
            .get(canister_role)
            .map(|declaration| declaration.package.clone())
            .ok_or_else(|| {
                ConfigOpsError::CanisterNotFound(
                    canister_role.to_string(),
                    "role declarations".to_string(),
                )
                .into()
            })
    }

    /// Resolve the explicit role-owned Fleet admission enrollment declaration.
    pub fn role_uses_fleet_admission(canister_role: &CanisterRole) -> Result<bool, InternalError> {
        let config = Config::get()?;
        config
            .role_uses_fleet_admission(canister_role)
            .ok_or_else(|| {
                ConfigOpsError::CanisterNotFound(
                    canister_role.to_string(),
                    "role declarations".to_string(),
                )
                .into()
            })
    }

    /// Resolve an implicit infrastructure role or a role structurally contained
    /// by exactly one Component Spec.
    pub fn try_get_canister_by_role(
        canister_role: &CanisterRole,
    ) -> Result<CanisterConfig, InternalError> {
        if canister_role.is_root() {
            return Ok(implicit_root_canister_config());
        }
        if canister_role.is_wasm_store() {
            return Ok(implicit_wasm_store_canister_config());
        }

        let component_spec = Self::try_get_component_spec_id_by_role(canister_role)?;
        Self::try_get_canister(&component_spec, canister_role)
    }

    /// Resolve the unique Component Spec structurally containing one role.
    fn try_get_component_spec_id_by_role(
        canister_role: &CanisterRole,
    ) -> Result<ComponentSpecId, InternalError> {
        let config = Config::get()?;
        let mut matches = config.component_specs_for_role(canister_role);
        let (component_spec, _component_spec_config) = matches.next().ok_or_else(|| {
            ConfigOpsError::CanisterNotFound(
                canister_role.to_string(),
                "Component Topology".to_string(),
            )
        })?;
        if matches.next().is_some() {
            return Err(ConfigOpsError::CanisterRoleAmbiguous(canister_role.to_string()).into());
        }

        Ok(component_spec.clone())
    }

    // ---------------------------------------------------------------------
    // Current-context / infallible helpers
    // ---------------------------------------------------------------------

    /// Return the immutable compiled App model to trusted control-plane validators.
    pub fn get() -> Result<Arc<ConfigModel>, InternalError> {
        let cfg = Config::get()?;

        Ok(cfg)
    }
}

/// Exact role-compiled runtime configuration authority.
pub struct ConfigOps;

impl ConfigOps {
    fn authority() -> Result<Arc<crate::config::RoleRuntimeAuthority>, InternalError> {
        if let Some(authority) = RoleRuntimeConfig::try_get() {
            return Ok(authority);
        }

        #[cfg(test)]
        {
            let config = Config::get()?;
            let role = EnvOps::canister_role().unwrap_or(CanisterRole::ROOT);
            let authority = if role.is_wasm_store() {
                crate::config::RoleRuntimeAuthority::compile_wasm_store(&config)
            } else {
                crate::config::RoleRuntimeAuthority::compile(&config, &role)
            }
            .map_err(|_error| InternalError::invariant())?;
            Ok(Arc::new(authority))
        }

        #[cfg(not(test))]
        {
            Err(InternalError::invariant())
        }
    }

    fn try_get_canister(
        component_spec: &ComponentSpecId,
        canister_role: &CanisterRole,
    ) -> Result<RuntimeCanisterConfig, InternalError> {
        Self::authority()?
            .canister(Some(component_spec), canister_role)
            .ok_or_else(|| {
                ConfigOpsError::CanisterNotFound(
                    canister_role.to_string(),
                    component_spec.to_string(),
                )
                .into()
            })
    }

    pub fn component_topology() -> Result<ComponentTopology, InternalError> {
        Ok(Self::authority()?.component_topology.clone())
    }

    pub fn validate_protected_component_deployment(
        context: &ProtectedComponentDeployment,
        owning_component: &ComponentBinding,
    ) -> Result<(), InternalError> {
        Self::authority()?.validate_protected_component_deployment(context, owning_component)
    }

    pub fn role_uses_fleet_admission(canister_role: &CanisterRole) -> Result<bool, InternalError> {
        let authority = Self::authority()?;
        (&authority.role == canister_role)
            .then_some(authority.fleet_admission)
            .ok_or_else(|| {
                ConfigOpsError::CanisterNotFound(
                    canister_role.to_string(),
                    "compiled runtime role".to_string(),
                )
                .into()
            })
    }

    pub fn try_get_canister_by_role(
        canister_role: &CanisterRole,
    ) -> Result<RuntimeCanisterConfig, InternalError> {
        Self::authority()?
            .canister_by_role(canister_role)
            .ok_or_else(|| ConfigOpsError::CanisterRoleAmbiguous(canister_role.to_string()).into())
    }

    pub(crate) fn log_config() -> Result<LogConfig, InternalError> {
        Ok(Self::authority()?.log.clone())
    }

    pub(crate) fn delegated_tokens_config() -> Result<DelegatedTokenConfig, InternalError> {
        Ok(Self::authority()?.auth.delegated_tokens.clone())
    }

    pub(crate) fn role_attestation_config() -> Result<RoleAttestationConfig, InternalError> {
        Ok(Self::authority()?.auth.role_attestation.clone())
    }

    pub(crate) fn local_application_authorization_for_role(
        role: &CanisterRole,
    ) -> Option<LocalApplicationAuthorizationConfig> {
        RoleRuntimeConfig::try_get()?.local_application_authorization(role)
    }

    pub(crate) fn app_init_mode() -> Result<FleetMode, InternalError> {
        let mode = match Self::authority()?.app_init_mode {
            FleetInitMode::Enabled => FleetMode::Enabled,
            FleetInitMode::Readonly => FleetMode::Readonly,
            FleetInitMode::Disabled => FleetMode::Disabled,
        };
        Ok(mode)
    }

    pub(crate) fn current_canister() -> Result<RuntimeCanisterConfig, InternalError> {
        let canister_role = EnvOps::canister_role()?;
        let component_spec = if canister_role.is_root() || canister_role.is_wasm_store() {
            None
        } else {
            Some(EnvOps::component_spec()?)
        };
        Self::authority()?
            .canister(component_spec.as_ref(), &canister_role)
            .ok_or_else(|| {
                ConfigOpsError::CanisterNotFound(
                    canister_role.to_string(),
                    component_spec.map_or_else(
                        || "infrastructure".to_string(),
                        |component_spec| component_spec.to_string(),
                    ),
                )
                .into()
            })
    }

    pub(crate) fn current_scaling_config() -> Result<Option<ScalingConfig>, InternalError> {
        Ok(Self::current_canister()?.scaling)
    }

    pub(crate) fn current_index_config() -> Result<Option<IndexConfig>, InternalError> {
        Ok(Self::current_canister()?.index)
    }

    pub(crate) fn current_component_canister(
        canister_role: &CanisterRole,
    ) -> Result<RuntimeCanisterConfig, InternalError> {
        let component_spec = EnvOps::component_spec()?;
        Self::try_get_canister(&component_spec, canister_role)
    }

    pub(crate) fn current_icrc21_enabled() -> bool {
        Self::authority().is_ok_and(|authority| {
            authority.global_icrc21
                && Self::current_canister().is_ok_and(|canister| canister.standards.icrc21)
        })
    }

    pub(crate) fn cycles_funding_limits_for_root_child_role(
        child_role: &CanisterRole,
    ) -> Result<FundingLimits, InternalError> {
        Ok(funding_limits(&Self::try_get_canister_by_role(child_role)?))
    }

    pub(crate) fn cycles_funding_limits_for_component_child_role(
        child_role: &CanisterRole,
    ) -> Result<FundingLimits, InternalError> {
        Ok(funding_limits(&Self::current_component_canister(
            child_role,
        )?))
    }
}

const fn funding_limits(cfg: &RuntimeCanisterConfig) -> FundingLimits {
    FundingLimits {
        max_per_request: cfg.cycles_funding.max_per_request.to_u128(),
        max_per_child: cfg.cycles_funding.max_per_child.to_u128(),
        cooldown_secs: cfg.cycles_funding.cooldown_secs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::schema::CanisterKind,
        storage::stable::env::{Env, EnvData, EnvRecord},
    };

    #[test]
    fn role_lookup_resolves_implicit_infrastructure_outside_component_topology() {
        let root = RootConfigOps::try_get_canister_by_role(&CanisterRole::ROOT)
            .expect("implicit root config");
        let wasm_store = RootConfigOps::try_get_canister_by_role(&CanisterRole::WASM_STORE)
            .expect("implicit Wasm Store config");

        assert_eq!(root.kind, CanisterKind::Root);
        assert_eq!(wasm_store.kind, CanisterKind::Singleton);
    }

    #[test]
    fn ordinary_runtime_lookup_requires_its_exact_component_spec() {
        Config::reset_for_tests();
        let config = ConfigModel::test_default();
        let role = CanisterRole::from("app");
        let authority = crate::config::RoleRuntimeAuthority::compile(&config, &role)
            .expect("compile ordinary runtime authority");
        RoleRuntimeConfig::init(authority).expect("install ordinary runtime authority");

        let original_env = Env::export();
        let component_spec =
            ComponentSpecId::try_from("default".to_string()).expect("default Component Spec ID");
        Env::import(EnvData {
            record: EnvRecord {
                canister_role: Some(role),
                component_spec: Some(component_spec),
                ..EnvRecord::default()
            },
        });
        ConfigOps::current_canister().expect("exact ordinary runtime config");

        let mut missing_component_spec = Env::export();
        missing_component_spec.record.component_spec = None;
        Env::import(missing_component_spec);
        assert!(ConfigOps::current_canister().is_err());

        Env::import(original_env);
        Config::reset_for_tests();
    }
}
