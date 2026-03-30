pub mod schema;

use crate::{InternalError, InternalErrorOrigin};
use schema::ConfigSchemaError;
use std::{cell::RefCell, sync::Arc};
use thiserror::Error as ThisError;

pub use schema::ConfigModel;
#[cfg(any(not(target_arch = "wasm32"), test))]
use schema::Validate;

//
// CONFIG
//
// Even though a canister executes single-threaded, there are a few practical reasons to favor Arc:
// APIs & trait bounds: Lots of ecosystem code (caches, services, executors, middleware) takes
// Arc<T> or requires Send + Sync. Rc<T> is neither Send nor Sync, so it won’t fit.
//
// Host-side tests & tools: The crate also builds for host targets (integration tests, benches,
// build scripts). Arc works across targets without cfg gymnastics.
//
// Globals need Sync: If config storage ever moves away from thread_local!, Arc<T> can participate.

struct InstalledConfig {
    model: Arc<ConfigModel>,
    source_toml: Arc<str>,
}

thread_local! {
    static CONFIG: RefCell<Option<InstalledConfig>> = const { RefCell::new(None) };
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

impl From<ConfigError> for InternalError {
    fn from(err: ConfigError) -> Self {
        Self::domain(InternalErrorOrigin::Config, err.to_string())
    }
}

///
/// Config
///

pub struct Config {}

impl Config {
    // Return the installed configuration model or initialize a test default when allowed.
    pub(crate) fn get() -> Result<Arc<ConfigModel>, InternalError> {
        CONFIG.with(|cfg| {
            if let Some(config) = cfg.borrow().as_ref() {
                return Ok(config.model.clone());
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

    // Return the installed configuration model when available.
    #[must_use]
    pub(crate) fn try_get() -> Option<Arc<ConfigModel>> {
        CONFIG.with(|cfg| {
            if let Some(config) = cfg.borrow().as_ref() {
                return Some(config.model.clone());
            }

            #[cfg(test)]
            {
                Some(Self::init_for_tests())
            }

            #[cfg(not(test))]
            {
                None
            }
        })
    }

    // Parse and validate a TOML configuration document on host targets.
    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub fn parse_toml(config_str: &str) -> Result<ConfigModel, ConfigError> {
        let config: ConfigModel =
            toml::from_str(config_str).map_err(|e| ConfigError::CannotParseToml(e.to_string()))?;

        config.validate().map_err(ConfigError::from)?;
        Ok(config)
    }

    // Install a trusted configuration model plus its canonical TOML source.
    pub(crate) fn init_from_model(
        config: ConfigModel,
        source_toml: &str,
    ) -> Result<Arc<ConfigModel>, ConfigError> {
        CONFIG.with(|cfg| {
            let mut borrow = cfg.borrow_mut();
            if borrow.is_some() {
                return Err(ConfigError::AlreadyInitialized);
            }

            let model = Arc::new(config);
            *borrow = Some(InstalledConfig {
                model: model.clone(),
                source_toml: Arc::<str>::from(source_toml),
            });

            Ok(model)
        })
    }

    // Test-only: initialize the global configuration from an in-memory model.
    #[cfg(test)]
    pub fn init_from_model_for_tests(config: ConfigModel) -> Result<Arc<ConfigModel>, ConfigError> {
        config.validate().map_err(ConfigError::from)?;

        let source_toml = toml::to_string_pretty(&config)
            .map_err(|e| ConfigError::CannotParseToml(e.to_string()))?;

        Self::init_from_model(config, &source_toml)
    }

    // Return the canonical TOML source embedded for the current configuration.
    pub(crate) fn to_toml() -> Result<String, InternalError> {
        CONFIG.with(|cfg| {
            cfg.borrow()
                .as_ref()
                .map(|config| config.source_toml.to_string())
                .ok_or_else(|| ConfigError::NotInitialized.into())
        })
    }

    // Test-only: reset the global config so tests can reinitialize with a fresh model.
    #[cfg(test)]
    pub fn reset_for_tests() {
        CONFIG.with(|cfg| {
            *cfg.borrow_mut() = None;
        });
    }

    // Test-only: ensure a minimal validated config is available.
    #[cfg(test)]
    #[must_use]
    pub fn init_for_tests() -> Arc<ConfigModel> {
        CONFIG.with(|cfg| {
            let mut borrow = cfg.borrow_mut();
            if let Some(existing) = borrow.as_ref() {
                return existing.model.clone();
            }

            let config = ConfigModel::test_default();
            config.validate().expect("test config must validate");

            let model = Arc::new(config);
            *borrow = Some(InstalledConfig {
                model: model.clone(),
                source_toml: Arc::<str>::from(""),
            });

            model
        })
    }
}
