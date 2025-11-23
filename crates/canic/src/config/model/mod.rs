mod log;
mod subnet;

pub use log::*;
pub use subnet::*;

use crate::{
    Error,
    config::ConfigError,
    types::{CanisterType, SubnetType},
};
use candid::Principal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error as ThisError;

///
/// ConfigModelError
///

#[derive(Debug, ThisError)]
pub enum ConfigModelError {
    #[error("subnet not found: {0}")]
    SubnetNotFound(SubnetType),

    #[error("canister not found on subnet: {0}")]
    CanisterNotFound(CanisterType),

    #[error("validation error: {0}")]
    ValidationError(String),
}

impl From<ConfigModelError> for Error {
    fn from(err: ConfigModelError) -> Self {
        ConfigError::from(err).into()
    }
}

///
/// Validate
///

pub trait Validate {
    fn validate(&self) -> Result<(), ConfigModelError>;
}

///
/// ConfigModel
///

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
    pub app_directory: BTreeSet<CanisterType>,

    #[serde(default)]
    pub subnets: BTreeMap<SubnetType, SubnetConfig>,

    #[serde(default)]
    pub whitelist: Option<Whitelist>,
}

impl ConfigModel {
    /// Get a subnet configuration by type.
    pub fn try_get_subnet(&self, ty: &SubnetType) -> Result<SubnetConfig, Error> {
        self.subnets
            .get(ty)
            .cloned()
            .ok_or_else(|| ConfigModelError::SubnetNotFound(ty.clone()).into())
    }

    /// Return true if the given principal is present in the whitelist.
    #[must_use]
    pub fn is_whitelisted(&self, principal: &Principal) -> bool {
        self.whitelist
            .as_ref()
            .is_none_or(|w| w.principals.contains(&principal.to_string()))
    }

    /// Return whether ICRC-21 standard support is enabled.
    #[must_use]
    pub fn icrc21_enabled(&self) -> bool {
        self.standards.as_ref().is_some_and(|s| s.icrc21)
    }
}

impl Validate for ConfigModel {
    fn validate(&self) -> Result<(), ConfigModelError> {
        //  Validate that prime subnet exists
        let prime = SubnetType::PRIME;
        let prime_subnet = self
            .subnets
            .get(&prime)
            .ok_or_else(|| ConfigModelError::ValidationError("prime subnet not found".into()))?;

        //  Validate that every app_directory entry exists in prime.canisters
        for canister_ty in &self.app_directory {
            if !prime_subnet.canisters.contains_key(canister_ty) {
                return Err(ConfigModelError::ValidationError(format!(
                    "app directory canister '{canister_ty}' is not in prime subnet",
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Whitelist {
    // principals
    // a hashset as we constantly have to do lookups
    // strings because then we can validate and know if there are any bad ones
    #[serde(default)]
    pub principals: BTreeSet<String>,
}

impl Validate for Whitelist {
    fn validate(&self) -> Result<(), ConfigModelError> {
        for (i, s) in self.principals.iter().enumerate() {
            if Principal::from_text(s).is_err() {
                return Err(ConfigModelError::ValidationError(format!(
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Standards {
    #[serde(default)]
    pub icrc21: bool,

    #[serde(default)]
    pub icrc103: bool,
}
