use crate::{
    Error,
    config::ConfigError,
    types::{CanisterType, Cycles, TC},
};
use candid::Principal;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use thiserror::Error as ThisError;

/// Errors encountered while validating or querying configuration data.
#[derive(Debug, ThisError)]
pub enum ConfigDataError {
    /// A principal string in the whitelist is invalid.
    #[error("invalid principal: {0} ({1})")]
    InvalidPrincipal(String, usize),

    /// A referenced canister type does not exist in the configuration.
    #[error("canister not found: {0}")]
    CanisterNotFound(CanisterType),
}

///
/// ConfigData
///

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigData {
    // controllers
    // a vec because we just append it to the controller arguments
    #[serde(default)]
    pub controllers: Vec<Principal>,

    #[serde(default)]
    pub canisters: HashMap<CanisterType, Canister>,

    #[serde(default)]
    pub whitelist: Option<WhiteList>,

    #[serde(default)]
    pub standards: Option<Standards>,

    #[serde(default)]
    pub pool: CanisterPool,

    #[serde(default)]
    pub cycle_tracker: bool,
}

impl ConfigData {
    pub(super) fn validate(&self) -> Result<(), ConfigDataError> {
        if let Some(list) = &self.whitelist {
            for (i, s) in list.principals.iter().enumerate() {
                // Reject if invalid principal format
                if Principal::from_text(s).is_err() {
                    return Err(ConfigDataError::InvalidPrincipal(s.to_string(), i));
                }
            }
        }

        Ok(())
    }

    /// Lookup a canister config by type name (string).
    pub fn get_canister(&self, ty: &CanisterType) -> Result<Canister, Error> {
        self.canisters.get(ty).cloned().ok_or_else(|| {
            ConfigError::ConfigDataError(ConfigDataError::CanisterNotFound(ty.clone())).into()
        })
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
/// Canister
///

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Canister {
    pub auto_create: Option<u16>,
    #[serde(deserialize_with = "Cycles::from_config")]
    pub initial_cycles: Cycles,
    pub topup: Option<CanisterTopup>,
    pub uses_directory: bool,
}

///
/// CanisterTopup
///
/// auto_topup : default to false
///

#[derive(Clone, Debug, Deserialize)]
pub struct CanisterTopup {
    #[serde(deserialize_with = "Cycles::from_config")]
    pub threshold: Cycles,

    #[serde(deserialize_with = "Cycles::from_config")]
    pub amount: Cycles,
}

impl Default for CanisterTopup {
    fn default() -> Self {
        Self {
            threshold: Cycles::new(10 * TC),
            amount: Cycles::new(5 * TC),
        }
    }
}

///
/// CanisterPool
/// defaults to a minimum size of 0
///

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CanisterPool {
    pub minimum_size: u8,
}

///
/// Whitelist
///

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhiteList {
    // principals
    // a hashset as we constantly have to do lookups
    // strings because then we can validate and know if there are any bad ones
    #[serde(default)]
    pub principals: HashSet<String>,
}

///
/// Standards
///

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Standards {
    #[serde(default)]
    pub icrc21: bool,
}
