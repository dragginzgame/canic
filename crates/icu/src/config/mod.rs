mod types;

use crate::Error;
use candid::{CandidType, Principal};
use serde::Deserialize;
use std::cell::RefCell;
use thiserror::Error as ThisError;

pub use types::ConfigData;

//
// CONFIG
//

thread_local! {
    static CONFIG: RefCell<ConfigData> = RefCell::new(ConfigData::default());
}

///
/// ConfigError
///

#[derive(CandidType, Debug, Deserialize, ThisError)]
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
    pub fn get() -> ConfigData {
        CONFIG.with_borrow(Clone::clone)
    }

    /// Initialize the global configuration from a TOML string.
    pub fn init_from_toml(config_str: &str) -> Result<(), Error> {
        let config: ConfigData =
            toml::from_str(config_str).map_err(|e| ConfigError::CannotParseToml(e.to_string()))?;

        Self::validate(&config)?;

        Self::set_config(config);

        Ok(())
    }

    /// Set the global configuration.
    fn set_config(config: ConfigData) {
        CONFIG.with(|cfg| {
            *cfg.borrow_mut() = config;
        });
    }

    fn validate(config: &ConfigData) -> Result<(), Error> {
        if let Some(ref wl) = config.whitelist {
            for (i, s) in wl.principals.iter().enumerate() {
                let s_trimmed = s.trim();

                // Reject if not ASCII
                if !s_trimmed.is_ascii() {
                    return Err(ConfigError::CannotParseToml(format!(
                        "Non-ASCII character in principal at index {i}: '{s_trimmed}'"
                    ))
                    .into());
                }

                // Reject if invalid principal format
                if let Err(e) = Principal::from_text(s_trimmed) {
                    return Err(ConfigError::CannotParseToml(format!(
                        "Invalid principal at index {i}: '{s_trimmed}' ({e})"
                    ))
                    .into());
                }
            }
        }
        Ok(())
    }
}
