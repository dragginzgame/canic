pub mod schema;

use crate::{Error, ThisError};
use schema::{ConfigSchemaError, Validate};
use std::{cell::RefCell, sync::Arc};

pub use schema::ConfigModel;

//
// CONFIG
//
// Even though a canister executes single‑threaded, there are a few practical reasons to favor Arc:
// APIs & trait bounds: Lots of ecosystem code (caches, services, executors, middleware) takes
// Arc<T> or requires Send + Sync. Rc<T> is neither Send nor Sync, so it won’t fit.
//
// Host-side tests & tools: Your crate likely builds for non‑wasm targets too (integration tests,
// benches, local tooling). Those can be multi‑threaded; Arc “just works” across targets without
// cfg gymnastics.
//
// Globals need Sync: If you ever move away from thread_local! or want to tuck the config behind
// a global static, Rc can’t participate; Arc<T> is Sync (when T: Send + Sync).
//

thread_local! {
    static CONFIG: RefCell<Option<Arc<ConfigModel>>> = const { RefCell::new(None) };
}

/// Errors related to configuration lifecycle and parsing.
#[derive(Debug, ThisError)]
pub enum ConfigError {
    #[error("config has already been initialized")]
    AlreadyInitialized,

    #[error("config has not been initialized")]
    NotInitialized,

    /// TOML could not be parsed into the expected structure.
    #[error("toml error: {0}")]
    CannotParseToml(String),

    /// Wrapper for data schema-level errors.
    #[error(transparent)]
    ConfigSchema(#[from] ConfigSchemaError),
}

///
/// Config
///

pub struct Config {}

impl Config {
    pub(crate) fn get() -> Result<Arc<ConfigModel>, Error> {
        CONFIG.with(|cfg| {
            if let Some(config) = cfg.borrow().as_ref() {
                return Ok(config.clone());
            }

            #[cfg(test)]
            {
                Ok(Self::init_for_tests())
            }

            #[cfg(not(test))]
            {
                Err(ConfigError::NotInitialized.into())
            }
        })
    }

    /// Initialize the global configuration from a TOML string.
    /// return the config as it is read at build time
    pub fn init_from_toml(config_str: &str) -> Result<Arc<ConfigModel>, ConfigError> {
        let config: ConfigModel =
            toml::from_str(config_str).map_err(|e| ConfigError::CannotParseToml(e.to_string()))?;

        // validate
        config.validate().map_err(ConfigError::from)?;

        CONFIG.with(|cfg| {
            let mut borrow = cfg.borrow_mut();
            if borrow.is_some() {
                return Err(ConfigError::AlreadyInitialized);
            }
            let arc = Arc::new(config);
            *borrow = Some(arc.clone());

            Ok(arc)
        })
    }

    /// Return the current config as a TOML string.
    pub(crate) fn to_toml() -> Result<String, Error> {
        let cfg = Self::get()?;

        toml::to_string_pretty(&*cfg)
            .map_err(|e| ConfigError::CannotParseToml(e.to_string()).into())
    }

    /// Test-only: reset the global config so tests can reinitialize with a fresh TOML.
    #[cfg(test)]
    pub fn reset_for_tests() {
        CONFIG.with(|cfg| {
            *cfg.borrow_mut() = None;
        });
    }

    /// Test-only: ensure a minimal validated config is available.
    #[cfg(test)]
    #[must_use]
    pub fn init_for_tests() -> Arc<ConfigModel> {
        CONFIG.with(|cfg| {
            let mut borrow = cfg.borrow_mut();
            if let Some(existing) = borrow.as_ref() {
                return existing.clone();
            }

            let config = ConfigModel::test_default();
            config.validate().expect("test config must validate");

            let arc = Arc::new(config);
            *borrow = Some(arc.clone());
            arc
        })
    }
}
