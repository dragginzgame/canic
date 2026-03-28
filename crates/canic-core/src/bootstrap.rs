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

/// Compact a validated Canic TOML source without changing value encodings.
#[must_use]
pub fn compact_config_source(toml: &str) -> String {
    let mut compact = String::new();

    for line in toml.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        compact.push_str(trimmed);
        compact.push('\n');
    }

    compact
}

fn parse_and_install_config(toml: &str) -> Result<Arc<ConfigModel>, ConfigError> {
    Config::init_from_toml(toml)?;

    Config::try_get().ok_or(ConfigError::NotInitialized)
}
