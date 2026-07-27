//! Module: ops::config
//!
//! Responsibility: expose fallible configuration lookups for ops and workflows.
//! Does not own: config parsing, environment initialization, or endpoint DTOs.
//! Boundary: ops layer between runtime context and immutable configuration model.

use crate::{
    InternalError,
    config::{
        ComponentTopology, Config, ConfigError, ConfigModel,
        schema::{
            CanisterConfig, ComponentSpecConfig, DelegatedTokenConfig, FleetInitMode, IndexConfig,
            LogConfig, RoleAttestationConfig, ScalingConfig, implicit_root_canister_config,
            implicit_wasm_store_canister_config,
        },
    },
    ids::{CanisterRole, ComponentSpecId},
    model::cycles_funding::FundingLimits,
    ops::{OpsError, prelude::*, runtime::env::EnvOps},
    storage::stable::state::fleet::FleetMode,
};
use std::{collections::BTreeSet, sync::Arc};
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
        OpsError::from(err).into()
    }
}

///
/// ConfigOps
///
/// Operations-layer facade for configuration access.
///

pub struct ConfigOps;

impl ConfigOps {
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
    pub(crate) fn try_get_component_spec_id_by_role(
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

    pub(crate) fn get() -> Result<Arc<ConfigModel>, InternalError> {
        let cfg = Config::get()?;

        Ok(cfg)
    }

    pub(crate) fn controllers() -> Result<Vec<Principal>, InternalError> {
        Ok(Config::get()?.controllers.clone())
    }

    pub(crate) fn is_whitelisted(caller: &Principal) -> Result<bool, InternalError> {
        Ok(Config::get()?.is_whitelisted(caller))
    }

    pub(crate) fn log_config() -> Result<LogConfig, InternalError> {
        Ok(Config::get()?.log.clone())
    }

    pub(crate) fn delegated_tokens_config() -> Result<DelegatedTokenConfig, InternalError> {
        Ok(Config::get()?.auth.delegated_tokens.clone())
    }

    pub(crate) fn role_attestation_config() -> Result<RoleAttestationConfig, InternalError> {
        Ok(Config::get()?.auth.role_attestation.clone())
    }

    pub(crate) fn app_init_mode() -> Result<FleetMode, InternalError> {
        let mode = match Config::get()?.app.init_mode {
            FleetInitMode::Enabled => FleetMode::Enabled,
            FleetInitMode::Readonly => FleetMode::Readonly,
            FleetInitMode::Disabled => FleetMode::Disabled,
        };

        Ok(mode)
    }

    /// Fetch the configuration record for the current Component.
    ///
    /// Requires that environment initialization has completed.
    pub fn current_component_spec() -> Result<ComponentSpecConfig, InternalError> {
        let component_spec = EnvOps::component_spec()?;

        Self::try_get_component_spec(&component_spec)
    }

    /// Resolve the exact role set visible through the current canister's
    /// root-local Subnet Directory projection.
    pub(crate) fn current_subnet_directory_roles() -> Result<BTreeSet<CanisterRole>, InternalError>
    {
        let canister_role = EnvOps::canister_role()?;
        if canister_role.is_wasm_store() {
            return Ok(BTreeSet::new());
        }
        if canister_role.is_root() {
            return Ok(Config::get()?.fleet_directory_roles());
        }

        Ok(Self::current_component_spec()?.component_directory_roles())
    }

    /// Fetch the configuration record for the *current* canister.
    pub(crate) fn current_canister() -> Result<CanisterConfig, InternalError> {
        let canister_role = EnvOps::canister_role()?;
        if canister_role.is_root() || canister_role.is_wasm_store() {
            return Self::try_get_canister_by_role(&canister_role);
        }
        let component_spec = EnvOps::component_spec()?;

        Self::try_get_canister(&component_spec, &canister_role)
    }

    /// Fetch the scaling configuration for the *current* canister.
    pub(crate) fn current_scaling_config() -> Result<Option<ScalingConfig>, InternalError> {
        Ok(Self::current_canister()?.scaling)
    }

    /// Fetch the keyed placement index config for the *current* canister.
    pub(crate) fn current_index_config() -> Result<Option<IndexConfig>, InternalError> {
        Ok(Self::current_canister()?.index)
    }

    /// Fetch the configuration for a role in the current Component Spec.
    pub(crate) fn current_component_canister(
        canister_role: &CanisterRole,
    ) -> Result<CanisterConfig, InternalError> {
        let component_spec = EnvOps::component_spec()?;

        Self::try_get_canister(&component_spec, canister_role)
    }

    /// Resolve the target canister configuration for one provisioning effect.
    ///
    /// A Fleet Subnet Root may create implicit infrastructure or a Component
    /// and therefore resolves the target globally. Any registered canister in
    /// a Component tree resolves a requested child role within that tree's
    /// protected Component Spec.
    pub(crate) fn canister_for_provisioning(
        canister_role: &CanisterRole,
    ) -> Result<CanisterConfig, InternalError> {
        if EnvOps::is_root() {
            Self::try_get_canister_by_role(canister_role)
        } else {
            Self::current_component_canister(canister_role)
        }
    }

    /// Resolve funding limits for a direct Fleet Subnet Root child role.
    ///
    /// Roots are infrastructure and have no current Component Spec, so this
    /// lookup resolves the unique topology role directly.
    pub(crate) fn cycles_funding_limits_for_root_child_role(
        child_role: &CanisterRole,
    ) -> Result<FundingLimits, InternalError> {
        let cfg = Self::try_get_canister_by_role(child_role)?;

        Ok(funding_limits(&cfg))
    }

    /// Resolve parent funding limits for a direct Component Child role.
    pub(crate) fn cycles_funding_limits_for_component_child_role(
        child_role: &CanisterRole,
    ) -> Result<FundingLimits, InternalError> {
        let cfg = Self::current_component_canister(child_role)?;

        Ok(funding_limits(&cfg))
    }
}

const fn funding_limits(cfg: &CanisterConfig) -> FundingLimits {
    FundingLimits {
        max_per_request: cfg.cycles_funding.max_per_request.to_u128(),
        max_per_child: cfg.cycles_funding.max_per_child.to_u128(),
        cooldown_secs: cfg.cycles_funding.cooldown_secs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::CanisterKind;

    #[test]
    fn role_lookup_resolves_implicit_infrastructure_outside_component_topology() {
        let root =
            ConfigOps::try_get_canister_by_role(&CanisterRole::ROOT).expect("implicit root config");
        let wasm_store = ConfigOps::try_get_canister_by_role(&CanisterRole::WASM_STORE)
            .expect("implicit Wasm Store config");

        assert_eq!(root.kind, CanisterKind::Root);
        assert_eq!(wasm_store.kind, CanisterKind::Singleton);
    }
}
