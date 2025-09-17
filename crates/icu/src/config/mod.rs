mod data;

use crate::{Error, types::CanisterType};
use data::{Canister, ConfigDataError};
use std::{cell::RefCell, sync::Arc};
use thiserror::Error as ThisError;

pub use data::ConfigData;

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
    static CONFIG: RefCell<Option<Arc<ConfigData>>> = const {  RefCell::new(None) };
}

/// Errors related to configuration lifecycle and parsing.
#[derive(Debug, ThisError)]
pub enum ConfigError {
    /// Configuration has already been initialized; reinitialization is not allowed.
    #[error("config has already been initialized")]
    AlreadyInitialized,

    /// TOML could not be parsed into the expected structure.
    #[error("toml error: {0}")]
    CannotParseToml(String),

    /// Configuration has not been initialized yet; call `icu_build!()` or `init_from_toml`.
    #[error("config has not been initialized")]
    NotInitialized,

    /// Wrapper for data-level errors.
    #[error(transparent)]
    ConfigDataError(#[from] ConfigDataError),
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
    /// return the config as it is read at build time
    pub fn init_from_toml(config_str: &str) -> Result<Arc<ConfigData>, Error> {
        let config: ConfigData =
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
        let cfg = Self::try_get()?;

        toml::to_string_pretty(&*cfg)
            .map_err(|e| ConfigError::CannotParseToml(e.to_string()).into())
    }

    // try_get_canister
    // helper function as its used everywhere
    pub fn try_get_canister(ty: &CanisterType) -> Result<Canister, Error> {
        let cfg = Self::try_get()?;
        cfg.try_get_canister(ty)
    }

    /// Test-only: reset the global config so tests can reinitialize with a fresh TOML.
    #[cfg(test)]
    pub fn reset_for_tests() {
        CONFIG.with(|cfg| {
            *cfg.borrow_mut() = None;
        });
    }
}
