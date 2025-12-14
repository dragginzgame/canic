use crate::{
    Error,
    config::{
        Config,
        schema::{CanisterConfig, SubnetConfig},
    },
    ids::{CanisterRole, SubnetRole},
    ops::{
        OpsError,
        model::memory::env::{EnvOps, EnvOpsError},
    },
};
use thiserror::Error as ThisError;

///
/// ConfigOpsError
///

#[derive(Debug, ThisError)]
pub enum ConfigOpsError {
    #[error("subnet {0} not found in configuration")]
    SubnetNotFound(String),

    #[error("canister {0} not defined in subnet {1}")]
    CanisterNotFound(String, String),

    #[error(transparent)]
    EnvOpsError(#[from] EnvOpsError),
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
/// - Provide fallible lookups over the config *model* (`try_get_subnet`,
///   `try_get_canister`).
/// - Provide "current context" helpers (`cfg_current_subnet`,
///   `cfg_current_canister`) that combine `EnvOps` (who am I?) with the
///   static configuration model.
///

pub struct ConfigOps;

impl ConfigOps {
    /// Fetch a subnet configuration by type.
    pub fn try_get_subnet(role: &SubnetRole) -> Result<SubnetConfig, Error> {
        let subnet_cfg = Config::get()
            .get_subnet(role)
            .ok_or_else(|| ConfigOpsError::SubnetNotFound(role.to_string()))?;

        Ok(subnet_cfg)
    }

    /// Get a canister configuration inside a specific subnet.
    pub fn try_get_canister(
        subnet_role: &SubnetRole,
        canister_role: &CanisterRole,
    ) -> Result<CanisterConfig, Error> {
        let subnet_cfg = Self::try_get_subnet(subnet_role)?;

        let canister_cfg = subnet_cfg.get_canister(canister_role).ok_or_else(|| {
            ConfigOpsError::CanisterNotFound(canister_role.to_string(), subnet_role.to_string())
        })?;

        Ok(canister_cfg)
    }

    /// Fetch the configuration record for the *current* subnet.
    pub fn current_subnet() -> Result<SubnetConfig, Error> {
        let subnet_role = EnvOps::try_get_subnet_role()?;

        // delegate lookup to ConfigOps
        let subnet_cfg = Self::try_get_subnet(&subnet_role)?;

        Ok(subnet_cfg)
    }

    pub fn current_canister() -> Result<CanisterConfig, Error> {
        let subnet_role = EnvOps::try_get_subnet_role()?;
        let canister_role = EnvOps::try_get_canister_role()?;

        // delegate lookup to ConfigOps or use subnet_cfg (either is fine)
        let canister_cfg = Self::try_get_canister(&subnet_role, &canister_role)?;

        Ok(canister_cfg)
    }

    pub fn current_subnet_canister(canister_role: &CanisterRole) -> Result<CanisterConfig, Error> {
        let subnet_role = EnvOps::try_get_subnet_role()?;

        // delegate lookup to ConfigOps or use subnet_cfg (either is fine)
        let canister_cfg = Self::try_get_canister(&subnet_role, canister_role)?;

        Ok(canister_cfg)
    }
}
