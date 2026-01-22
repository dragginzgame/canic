use crate::config::{Config, ConfigError, schema::ConfigModel};
use std::sync::Arc;

/// Parse, validate, and install the Canic configuration.
///
/// This function is intentionally:
/// - deterministic
/// - synchronous
/// - side-effect limited to internal config state
///
/// It is safe to call from both build-time validation and
/// canister init / post-upgrade bootstrap paths.
pub fn init_config(toml: &str) -> Result<Arc<ConfigModel>, ConfigError> {
    parse_and_install_config(toml)
}

fn parse_and_install_config(toml: &str) -> Result<Arc<ConfigModel>, ConfigError> {
    Config::init_from_toml(toml)?;

    Config::try_get().ok_or(ConfigError::NotInitialized)
}
