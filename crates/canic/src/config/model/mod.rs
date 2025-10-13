mod subnet;

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
    #[error("invalid principal: {0} ({1})")]
    InvalidPrincipal(String, usize),

    #[error("subnet not found: {0}")]
    SubnetNotFound(SubnetType),

    #[error("canister not found on subnet: {0}")]
    CanisterNotFound(CanisterType),

    #[error("app_directory canister not found on prime subnet: {0}")]
    MissingAppDirectoryCanister(CanisterType),
}

impl From<ConfigModelError> for Error {
    fn from(err: ConfigModelError) -> Self {
        ConfigError::from(err).into()
    }
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
    pub reserve: CanisterReserve,

    #[serde(default)]
    pub standards: Option<Standards>,

    #[serde(default)]
    pub app_directory: BTreeSet<CanisterType>,

    #[serde(default)]
    pub subnets: BTreeMap<SubnetType, SubnetConfig>,

    #[serde(default)]
    pub whitelist: Option<WhiteList>,
}

impl ConfigModel {
    pub(super) fn validate(&self) -> Result<(), ConfigModelError> {
        // 1. Validate whitelist principals
        if let Some(list) = &self.whitelist {
            for (i, s) in list.principals.iter().enumerate() {
                if Principal::from_text(s).is_err() {
                    return Err(ConfigModelError::InvalidPrincipal(s.to_string(), i));
                }
            }
        }

        // 2. Validate that prime subnet exists
        let prime = SubnetType::PRIME;
        let prime_subnet = self
            .subnets
            .get(&prime)
            .ok_or_else(|| ConfigModelError::SubnetNotFound(prime.clone()))?;

        // 3. Validate that every app_directory entry exists in prime.canisters
        for canister_ty in &self.app_directory {
            if !prime_subnet.canisters.contains_key(canister_ty) {
                return Err(ConfigModelError::MissingAppDirectoryCanister(
                    canister_ty.clone(),
                ));
            }
        }

        Ok(())
    }

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

///
/// CanisterReserve
/// defaults to a minimum size of 0
///

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CanisterReserve {
    pub minimum_size: u8,
}

///
/// Whitelist
///

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhiteList {
    // principals
    // a hashset as we constantly have to do lookups
    // strings because then we can validate and know if there are any bad ones
    #[serde(default)]
    pub principals: BTreeSet<String>,
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
