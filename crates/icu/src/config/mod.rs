mod types;

use crate::Error;
use candid::Principal;
use std::{cell::RefCell, sync::Arc};
use thiserror::Error as ThisError;

pub use types::ConfigData;

//
// CONFIG
//

thread_local! {
    static CONFIG: RefCell<Option<Arc<ConfigData>>> = const {  RefCell::new(None) };
}

///
/// ConfigError
///

#[derive(Debug, ThisError)]
pub enum ConfigError {
    #[error("config has already been initialized")]
    AlreadyInitialized,

    #[error("toml error: {0}")]
    CannotParseToml(String),

    #[error("invalid principal: {0} ({1})")]
    InvalidPrincipal(String, usize),

    #[error("config has not been initialized")]
    NotInitialized,
}

///
/// Config
///

pub struct Config {}

impl Config {
    // use an Arc to avoid repeatedly cloning
    pub fn try_get() -> Result<Arc<ConfigData>, Error> {
        let arc = CONFIG.with(|cfg| {
            cfg.borrow()
                .as_ref()
                .cloned()
                .ok_or(ConfigError::NotInitialized)
        })?;

        Ok(arc)
    }

    /// Initialize the global configuration from a TOML string.
    pub fn init_from_toml(config_str: &str) -> Result<(), Error> {
        let config: ConfigData =
            toml::from_str(config_str).map_err(|e| ConfigError::CannotParseToml(e.to_string()))?;

        // validate
        Self::validate(&config)?;

        CONFIG.with(|cfg| {
            let mut borrow = cfg.borrow_mut();
            if borrow.is_some() {
                return Err(ConfigError::AlreadyInitialized.into());
            }
            *borrow = Some(Arc::new(config));

            Ok(())
        })
    }

    fn validate(config: &ConfigData) -> Result<(), Error> {
        if let Some(list) = &config.whitelist {
            for (i, s) in list.principals.iter().enumerate() {
                // Reject if invalid principal format
                if Principal::from_text(s).is_err() {
                    return Err(ConfigError::InvalidPrincipal(s.to_string(), i).into());
                }
            }
        }

        Ok(())
    }
}
