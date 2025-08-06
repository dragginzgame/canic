use crate::Error;
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashSet};
use thiserror::Error as ThisError;

//
// CONFIG
//

thread_local! {
    static CONFIG: RefCell<ConfigData> = RefCell::new(ConfigData::default());
    static CONFIG_INITIALIZED: RefCell<bool> = const { RefCell::new(false) };
}

///
/// ConfigError
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum ConfigError {
    #[error("config has already been initialized")]
    AlreadyInitialized,

    #[error("toml error: {0}")]
    CannotParseToml(String),

    #[error("config not initialized")]
    NotInitialized,
}

///
/// Config
///

pub struct Config {}

impl Config {
    /// Get the global config data.
    pub fn get() -> Result<ConfigData, Error> {
        let is_initialized = CONFIG_INITIALIZED.with(|flag| *flag.borrow());

        if !is_initialized {
            return Err(ConfigError::NotInitialized)?;
        }

        Ok(CONFIG.with_borrow(|cfg| cfg.clone()))
    }

    /// Initialize the global configuration from a TOML string.
    pub fn init_from_toml(config_str: &str) -> Result<(), Error> {
        let config: ConfigData =
            toml::from_str(config_str).map_err(|e| ConfigError::CannotParseToml(e.to_string()))?;

        Self::set_config(config)?;

        Ok(())
    }

    /// Set the global configuration.
    fn set_config(config: ConfigData) -> Result<(), ConfigError> {
        CONFIG_INITIALIZED.with_borrow_mut(|flag| {
            if *flag {
                return Err(ConfigError::AlreadyInitialized);
            }

            // Write the config and mark as initialized
            CONFIG.with(|cfg| {
                *cfg.borrow_mut() = config;
            });

            *flag = true;

            Ok(())
        })
    }
}

///
/// ConfigData
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct ConfigData {
    pub controllers: HashSet<Principal>,
    pub whitelist: HashSet<Principal>,
}
