use crate::{
    Error, ThisError,
    config::{
        Config, ConfigModel,
        schema::{CanisterConfig, LogConfig, ScalingConfig, SubnetConfig},
    },
    ids::SubnetRole,
    ops::{OpsError, prelude::*, runtime::env},
};
use std::sync::Arc;

pub mod network;

///
/// ConfigOpsError
///

#[derive(Debug, ThisError)]
pub enum ConfigOpsError {
    #[error("subnet {0} not found in configuration")]
    SubnetNotFound(String),

    #[error("canister {0} not defined in subnet {1}")]
    CanisterNotFound(String, String),
}

impl From<ConfigOpsError> for Error {
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
    pub(crate) fn export_toml() -> Result<String, Error> {
        Config::to_toml()
    }

    // ---------------------------------------------------------------------
    // Explicit / fallible lookups
    // ---------------------------------------------------------------------

    /// Fetch a subnet configuration by role.
    pub(crate) fn try_get_subnet(role: &SubnetRole) -> Result<SubnetConfig, Error> {
        let cfg = Config::get()?;

        cfg.get_subnet(role)
            .ok_or_else(|| ConfigOpsError::SubnetNotFound(role.to_string()).into())
    }

    /// Fetch a canister configuration within a specific subnet.
    pub(crate) fn try_get_canister(
        subnet_role: &SubnetRole,
        canister_role: &CanisterRole,
    ) -> Result<CanisterConfig, Error> {
        let subnet_cfg = Self::try_get_subnet(subnet_role)?;

        subnet_cfg.get_canister(canister_role).ok_or_else(|| {
            ConfigOpsError::CanisterNotFound(canister_role.to_string(), subnet_role.to_string())
                .into()
        })
    }

    // ---------------------------------------------------------------------
    // Current-context / infallible helpers
    // ---------------------------------------------------------------------

    pub(crate) fn get() -> Result<Arc<ConfigModel>, Error> {
        Config::get()
    }

    pub(crate) fn controllers() -> Result<Vec<Principal>, Error> {
        Ok(Config::get()?.controllers.clone())
    }

    pub(crate) fn log_config() -> Result<LogConfig, Error> {
        Ok(Config::get()?.log.clone())
    }

    /// Fetch the configuration record for the *current* subnet.
    ///
    /// Requires that environment initialization has completed.
    pub(crate) fn current_subnet() -> Result<SubnetConfig, Error> {
        let subnet_role = env::subnet_role()?;

        Self::try_get_subnet(&subnet_role)
    }

    /// Fetch the configuration record for the *current* canister.
    pub(crate) fn current_canister() -> Result<CanisterConfig, Error> {
        let subnet_role = env::subnet_role()?;
        let canister_role = env::canister_role()?;

        Self::try_get_canister(&subnet_role, &canister_role)
    }

    /// Fetch the scaling configuration for the *current* canister.
    pub(crate) fn current_scaling_config() -> Result<Option<ScalingConfig>, Error> {
        Ok(Self::current_canister()?.scaling)
    }

    /// Fetch the configuration for a specific canister in the *current* subnet.
    pub(crate) fn current_subnet_canister(
        canister_role: &CanisterRole,
    ) -> Result<CanisterConfig, Error> {
        let subnet_role = env::subnet_role()?;

        Self::try_get_canister(&subnet_role, canister_role)
    }
}
