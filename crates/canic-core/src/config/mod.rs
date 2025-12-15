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

    /// TOML could not be parsed into the expected structure.
    #[error("toml error: {0}")]
    CannotParseToml(String),

    /// Wrapper for data schema-level errors.
    #[error(transparent)]
    ConfigSchemaError(#[from] ConfigSchemaError),
}

///
/// Config
///

pub struct Config {}

impl Config {
    // use an Arc to avoid repeatedly cloning
    #[must_use]
    pub fn get() -> Arc<ConfigModel> {
        CONFIG.with(|cfg| {
            cfg.borrow()
                .as_ref()
                .cloned()
                .expect("⚠️ Config must be initialized before use")
        })
    }

    /// Return the config if initialized, otherwise `None`.
    #[must_use]
    pub fn try_get() -> Option<Arc<ConfigModel>> {
        CONFIG.with(|cfg| cfg.borrow().as_ref().cloned())
    }

    /// Initialize the global configuration from a TOML string.
    /// return the config as it is read at build time
    pub fn init_from_toml(config_str: &str) -> Result<Arc<ConfigModel>, Error> {
        let config: ConfigModel =
            toml::from_str(config_str).map_err(|e| ConfigError::CannotParseToml(e.to_string()))?;

        // validate
        config.validate().map_err(ConfigError::from)?;

        CONFIG.with(|cfg| {
            let mut borrow = cfg.borrow_mut();
            if borrow.is_some() {
                return Err(ConfigError::AlreadyInitialized.into());
            }
            let arc = Arc::new(config);
            *borrow = Some(arc.clone());

            Ok(arc)
        })
    }

    /// Return the current config as a TOML string.
    pub fn to_toml() -> Result<String, Error> {
        let cfg = Self::get();

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
}
