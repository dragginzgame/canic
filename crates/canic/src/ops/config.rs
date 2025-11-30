use crate::{
    Error,
    config::{
        Config,
        schema::{CanisterConfig, SubnetConfig},
    },
    ops::{
        OpsError,
        model::memory::env::{EnvOps, EnvOpsError},
    },
    types::{CanisterType, SubnetType},
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
    pub fn try_get_subnet(ty: &SubnetType) -> Result<SubnetConfig, Error> {
        let subnet_cfg = Config::get()
            .get_subnet(ty)
            .ok_or(ConfigOpsError::SubnetNotFound(ty.to_string()))?;

        Ok(subnet_cfg)
    }

    /// Get a canister configuration inside a specific subnet.
    pub fn try_get_canister(
        subnet_type: &SubnetType,
        canister_type: &CanisterType,
    ) -> Result<CanisterConfig, Error> {
        let subnet_cfg = Self::try_get_subnet(subnet_type)?;

        let canister_cfg = subnet_cfg.get_canister(canister_type).ok_or_else(|| {
            ConfigOpsError::CanisterNotFound(canister_type.to_string(), subnet_type.to_string())
        })?;

        Ok(canister_cfg)
    }

    /// Fetch the configuration record for the *current* subnet.
    pub fn current_subnet() -> Result<SubnetConfig, Error> {
        let subnet_type = EnvOps::try_get_subnet_type()?;

        // delegate lookup to ConfigOps
        let subnet_cfg = Self::try_get_subnet(&subnet_type)?;

        Ok(subnet_cfg)
    }

    pub fn current_canister() -> Result<CanisterConfig, Error> {
        let subnet_type = EnvOps::try_get_subnet_type()?;
        let canister_type = EnvOps::try_get_canister_type()?;

        // delegate lookup to ConfigOps or use subnet_cfg (either is fine)
        let canister_cfg = ConfigOps::try_get_canister(&subnet_type, &canister_type)?;

        Ok(canister_cfg)
    }

    pub fn current_subnet_canister(canister_type: &CanisterType) -> Result<CanisterConfig, Error> {
        let subnet_type = EnvOps::try_get_subnet_type()?;

        // delegate lookup to ConfigOps or use subnet_cfg (either is fine)
        let canister_cfg = ConfigOps::try_get_canister(&subnet_type, &canister_type)?;

        Ok(canister_cfg)
    }
}
