use crate::{
    InternalError,
    config::{
        Config, ConfigError, ConfigModel,
        schema::{
            AppInitMode, CanisterConfig, DelegationConfig, LogConfig, ScalingConfig, SubnetConfig,
        },
    },
    ids::SubnetRole,
    ops::{OpsError, prelude::*, runtime::env::EnvOps},
    storage::stable::state::app::AppMode,
};
use std::sync::Arc;
use thiserror::Error as ThisError;

///
/// ConfigOpsError
///

#[derive(Debug, ThisError)]
pub enum ConfigOpsError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error("subnet {0} not found in configuration")]
    SubnetNotFound(String),

    #[error("canister {0} not defined in subnet {1}")]
    CanisterNotFound(String, String),
}

impl From<ConfigOpsError> for InternalError {
    fn from(err: ConfigOpsError) -> Self {
        OpsError::from(err).into()
    }
}

///
/// ConfigOps
///
/// Ops-layer façade for configuration access.
///
/// Responsibilities:
/// - Provide fallible lookups over the configuration model (`try_get_*`)
/// - Provide infallible access to the *current* subnet/canister context,
///   assuming environment initialization has completed
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

    /// Fetch a subnet configuration by role.
    pub(crate) fn try_get_subnet(role: &SubnetRole) -> Result<SubnetConfig, InternalError> {
        let cfg = Config::get()?;

        cfg.get_subnet(role)
            .ok_or_else(|| ConfigOpsError::SubnetNotFound(role.to_string()).into())
    }

    /// Fetch a canister configuration within a specific subnet.
    pub(crate) fn try_get_canister(
        subnet_role: &SubnetRole,
        canister_role: &CanisterRole,
    ) -> Result<CanisterConfig, InternalError> {
        let subnet_cfg = Self::try_get_subnet(subnet_role)?;

        subnet_cfg.get_canister(canister_role).ok_or_else(|| {
            ConfigOpsError::CanisterNotFound(canister_role.to_string(), subnet_role.to_string())
                .into()
        })
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

    pub(crate) fn log_config() -> Result<LogConfig, InternalError> {
        Ok(Config::get()?.log.clone())
    }

    pub(crate) fn delegation_config() -> Result<DelegationConfig, InternalError> {
        Ok(Config::get()?.delegation.clone())
    }

    pub(crate) fn app_init_mode() -> Result<AppMode, InternalError> {
        let mode = match Config::get()?.app.init_mode {
            AppInitMode::Enabled => AppMode::Enabled,
            AppInitMode::Readonly => AppMode::Readonly,
            AppInitMode::Disabled => AppMode::Disabled,
        };

        Ok(mode)
    }

    /// Fetch the configuration record for the *current* subnet.
    ///
    /// Requires that environment initialization has completed.
    pub(crate) fn current_subnet() -> Result<SubnetConfig, InternalError> {
        let subnet_role = EnvOps::subnet_role()?;

        Self::try_get_subnet(&subnet_role)
    }

    /// Fetch the configuration record for the *current* canister.
    pub(crate) fn current_canister() -> Result<CanisterConfig, InternalError> {
        let subnet_role = EnvOps::subnet_role()?;
        let canister_role = EnvOps::canister_role()?;

        Self::try_get_canister(&subnet_role, &canister_role)
    }

    /// Fetch the scaling configuration for the *current* canister.
    pub(crate) fn current_scaling_config() -> Result<Option<ScalingConfig>, InternalError> {
        Ok(Self::current_canister()?.scaling)
    }

    /// Fetch the configuration for a specific canister in the *current* subnet.
    pub(crate) fn current_subnet_canister(
        canister_role: &CanisterRole,
    ) -> Result<CanisterConfig, InternalError> {
        let subnet_role = EnvOps::subnet_role()?;

        Self::try_get_canister(&subnet_role, canister_role)
    }
}
