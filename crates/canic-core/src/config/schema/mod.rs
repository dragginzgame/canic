mod log;
mod subnet;

pub use log::*;
pub use subnet::*;

use crate::{
    Error, ThisError,
    config::ConfigError,
    ids::{CanisterRole, SubnetRole},
};
use candid::Principal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

///
/// ConfigSchemaError
///

#[derive(Debug, ThisError)]
pub enum ConfigSchemaError {
    #[error("validation error: {0}")]
    ValidationError(String),
}

pub const NAME_MAX_BYTES: usize = 40;

fn validate_canister_role_len(role: &CanisterRole, context: &str) -> Result<(), ConfigSchemaError> {
    if role.as_ref().len() > NAME_MAX_BYTES {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{context} '{role}' exceeds {NAME_MAX_BYTES} bytes",
        )));
    }

    Ok(())
}

fn validate_subnet_role_len(role: &SubnetRole, context: &str) -> Result<(), ConfigSchemaError> {
    if role.as_ref().len() > NAME_MAX_BYTES {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{context} '{role}' exceeds {NAME_MAX_BYTES} bytes",
        )));
    }

    Ok(())
}

impl From<ConfigSchemaError> for Error {
    fn from(err: ConfigSchemaError) -> Self {
        ConfigError::from(err).into()
    }
}

///
/// Validate
///

pub trait Validate {
    fn validate(&self) -> Result<(), ConfigSchemaError>;
}

///
/// ConfigModel
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigModel {
    // controllers
    // a vec because we just append it to the controller arguments
    #[serde(default)]
    pub controllers: Vec<Principal>,

    #[serde(default)]
    pub standards: Option<Standards>,

    #[serde(default)]
    pub log: LogConfig,

    #[serde(default)]
    pub app_directory: BTreeSet<CanisterRole>,

    #[serde(default)]
    pub subnets: BTreeMap<SubnetRole, SubnetConfig>,

    #[serde(default)]
    pub whitelist: Option<Whitelist>,
}

impl ConfigModel {
    /// Get a subnet configuration by role.
    #[must_use]
    pub fn get_subnet(&self, role: &SubnetRole) -> Option<SubnetConfig> {
        self.subnets.get(role).cloned()
    }

    /// Test-only: baseline config with a prime subnet so validation succeeds.
    #[cfg(test)]
    #[must_use]
    pub fn test_default() -> Self {
        let mut cfg = Self::default();
        cfg.subnets
            .insert(SubnetRole::PRIME, SubnetConfig::default());
        cfg
    }

    /// Return true if the given principal is present in the whitelist.
    #[must_use]
    pub fn is_whitelisted(&self, principal: &Principal) -> bool {
        self.whitelist
            .as_ref()
            .is_none_or(|w| w.principals.contains(&principal.to_string()))
    }
}

impl Validate for ConfigModel {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        for subnet_role in self.subnets.keys() {
            validate_subnet_role_len(subnet_role, "subnet")?;
        }

        self.log.validate()?;

        // Validate that prime subnet exists
        let prime = SubnetRole::PRIME;
        let prime_subnet = self
            .subnets
            .get(&prime)
            .ok_or_else(|| ConfigSchemaError::ValidationError("prime subnet not found".into()))?;

        // Validate that every app_directory entry exists in prime.canisters
        for canister_role in &self.app_directory {
            validate_canister_role_len(canister_role, "app directory canister")?;
            let canister_cfg = prime_subnet.canisters.get(canister_role).ok_or_else(|| {
                ConfigSchemaError::ValidationError(format!(
                    "app directory canister '{canister_role}' is not in prime subnet",
                ))
            })?;

            if canister_cfg.cardinality != CanisterCardinality::Single {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "app directory canister '{canister_role}' must have cardinality = \"single\"",
                )));
            }
        }

        // child validation
        if let Some(list) = &self.whitelist {
            list.validate()?;
        }
        for subnet in self.subnets.values() {
            subnet.validate()?;
        }

        Ok(())
    }
}

///
/// Whitelist
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Whitelist {
    // principals
    // a hashset as we constantly have to do lookups
    // strings because then we can validate and know if there are any bad ones
    #[serde(default)]
    pub principals: BTreeSet<String>,
}

impl Validate for Whitelist {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        for (i, s) in self.principals.iter().enumerate() {
            if Principal::from_text(s).is_err() {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "principal #{i} {s} is invalid"
                )));
            }
        }

        Ok(())
    }
}

///
/// Standards
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Standards {
    #[serde(default)]
    pub icrc21: bool,

    #[serde(default)]
    pub icrc103: bool,
}
