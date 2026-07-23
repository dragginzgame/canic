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
#[cfg(any(not(target_arch = "wasm32"), test))]
use serde_path_to_error::{Path as SerdePath, Segment as SerdePathSegment};

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
    #[error("toml {issue}: {detail}")]
    CannotParseToml {
        issue: ConfigTomlIssue,
        detail: String,
    },

    /// Wrapper for data schema-level errors.
    #[error(transparent)]
    ConfigSchema(#[from] ConfigSchemaError),

    /// Runtime root-key injection failed during local/test bootstrap.
    #[error("runtime IC root key error: {0}")]
    RuntimeRootKey(String),
}

/// Structured classification for a TOML parsing failure.
#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum ConfigTomlIssue {
    #[error("document is invalid")]
    InvalidDocument,

    #[error("value at {logical_path} is invalid")]
    InvalidValue { logical_path: String },

    #[error("contains unknown field {unknown_field} at {logical_path}")]
    UnknownField {
        logical_path: String,
        unknown_field: String,
    },
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

pub struct Config;

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
        let deserializer = toml::Deserializer::parse(config_str).map_err(|source| {
            ConfigError::CannotParseToml {
                issue: ConfigTomlIssue::InvalidDocument,
                detail: source.to_string(),
            }
        })?;
        let config: ConfigModel =
            serde_path_to_error::deserialize(deserializer).map_err(|error| {
                let issue = classify_toml_issue(config_str, error.path(), error.inner());
                ConfigError::CannotParseToml {
                    issue,
                    detail: error.into_inner().to_string(),
                }
            })?;

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

        let source_toml =
            toml::to_string_pretty(&config).map_err(|source| ConfigError::CannotParseToml {
                issue: ConfigTomlIssue::InvalidDocument,
                detail: source.to_string(),
            })?;

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

#[cfg(any(not(target_arch = "wasm32"), test))]
fn classify_toml_issue(
    source_toml: &str,
    path: &SerdePath,
    source: &toml::de::Error,
) -> ConfigTomlIssue {
    let logical_path = path.to_string();
    let unknown_field = path.iter().next_back().and_then(|segment| match segment {
        SerdePathSegment::Map { key } => Some(key),
        SerdePathSegment::Seq { .. }
        | SerdePathSegment::Enum { .. }
        | SerdePathSegment::Unknown => None,
    });
    if let (Some(unknown_field), Some(span)) = (unknown_field, source.span())
        && source_toml
            .get(span)
            .is_some_and(|token| token == unknown_field)
    {
        return ConfigTomlIssue::UnknownField {
            logical_path,
            unknown_field: unknown_field.clone(),
        };
    }

    if logical_path == "." {
        ConfigTomlIssue::InvalidDocument
    } else {
        ConfigTomlIssue::InvalidValue { logical_path }
    }
}
