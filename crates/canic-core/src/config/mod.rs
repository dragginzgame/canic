//! Module: config
//!
//! Responsibility: own runtime installation and host parsing of Canic configuration.
//! Does not own: schema field definitions, validation rules, or endpoint DTOs.
//! Boundary: bootstrap installs validated config here before ops/workflow reads it.

pub mod schema;
#[cfg(any(not(target_arch = "wasm32"), test))]
mod validation;

use crate::{InternalError, InternalErrorOrigin};
use schema::ConfigSchemaError;
use std::{cell::RefCell, sync::Arc};
use thiserror::Error as ThisError;

pub use schema::ConfigModel;
#[cfg(any(not(target_arch = "wasm32"), test))]
use schema::Validate;

struct InstalledConfig {
    model: Arc<ConfigModel>,
    source_toml: Arc<str>,
}

thread_local! {
    static CONFIG: RefCell<Option<InstalledConfig>> = const { RefCell::new(None) };
}

///
/// ConfigError
///
/// Configuration lifecycle, parsing, validation, and runtime injection error.
/// Owned by config and converted into `InternalError` at config boundaries.
///

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

    /// Runtime root-key injection failed during local/test bootstrap.
    #[error("runtime IC root key error: {0}")]
    RuntimeRootKey(String),
}

impl From<ConfigError> for InternalError {
    fn from(err: ConfigError) -> Self {
        Self::domain(InternalErrorOrigin::Config, err.to_string())
    }
}

///
/// Config
///
/// Runtime config installation and lookup facade.
/// Owned by config and used by bootstrap, ops, and tests.
///

pub struct Config {}

impl Config {
    /// Return the installed configuration model or initialize a test default when allowed.
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

    /// Return the installed configuration model when available.
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

    /// Parse and validate a TOML configuration document on host targets.
    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub fn parse_toml(config_str: &str) -> Result<ConfigModel, ConfigError> {
        let config: ConfigModel =
            toml::from_str(config_str).map_err(|e| ConfigError::CannotParseToml(e.to_string()))?;

        config.validate().map_err(ConfigError::from)?;
        Ok(config)
    }

    /// Install a trusted configuration model plus its canonical TOML source.
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

    /// Initialize the global configuration from an in-memory model for tests.
    #[cfg(test)]
    pub fn init_from_model_for_tests(config: ConfigModel) -> Result<Arc<ConfigModel>, ConfigError> {
        config.validate().map_err(ConfigError::from)?;

        let source_toml = toml::to_string_pretty(&config)
            .map_err(|e| ConfigError::CannotParseToml(e.to_string()))?;

        Self::init_from_model(config, &source_toml)
    }

    /// Return the canonical TOML source embedded for the current configuration.
    pub(crate) fn to_toml() -> Result<String, InternalError> {
        CONFIG.with(|cfg| {
            cfg.borrow()
                .as_ref()
                .map(|config| config.source_toml.to_string())
                .ok_or_else(|| ConfigError::NotInitialized.into())
        })
    }

    /// Reset the global config so tests can reinitialize with a fresh model.
    #[cfg(test)]
    pub fn reset_for_tests() {
        CONFIG.with(|cfg| {
            *cfg.borrow_mut() = None;
        });
    }

    /// Ensure a minimal validated config is available for tests.
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
