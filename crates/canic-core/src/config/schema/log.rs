//! Module: config::schema::log
//!
//! Responsibility: define log configuration schema and validation limits.
//! Does not own: log storage, runtime metrics, or operator rendering.
//! Boundary: config schema re-exports this data for validated config models.

use serde::{Deserialize, Serialize};

#[cfg(any(not(target_arch = "wasm32"), test))]
use super::{ConfigSchemaError, Validate};

mod defaults {
    pub const fn max_entries() -> u64 {
        10_000
    }

    pub const fn max_entry_bytes() -> u32 {
        16_384
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
pub const MAX_LOG_ENTRIES: u64 = 100_000;

#[cfg(any(not(target_arch = "wasm32"), test))]
const MAX_LOG_ENTRY_BYTES: u32 = 65_536;

///
/// LogConfig
///
/// Log retention and entry-size configuration.
/// Owned by config schema and validated before runtime installation.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogConfig {
    #[serde(default = "defaults::max_entries")]
    pub max_entries: u64,

    #[serde(default = "defaults::max_entry_bytes")]
    pub max_entry_bytes: u32,

    #[serde(default)]
    pub max_age_secs: Option<u64>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            max_entries: defaults::max_entries(),
            max_entry_bytes: defaults::max_entry_bytes(),
            max_age_secs: None,
        }
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl Validate for LogConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        if self.max_entries > MAX_LOG_ENTRIES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "log.max_entries {} exceeds max {}",
                self.max_entries, MAX_LOG_ENTRIES
            )));
        }
        if self.max_entry_bytes > MAX_LOG_ENTRY_BYTES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "log.max_entry_bytes {} exceeds max {}",
                self.max_entry_bytes, MAX_LOG_ENTRY_BYTES
            )));
        }

        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_message_size_accepts_defaults_and_exact_boundaries() {
        assert!(LogConfig::default().validate().is_ok());
        for max_entry_bytes in [0, MAX_LOG_ENTRY_BYTES] {
            let config = LogConfig {
                max_entry_bytes,
                ..LogConfig::default()
            };
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn log_message_size_rejects_values_above_the_bound() {
        for max_entry_bytes in [MAX_LOG_ENTRY_BYTES + 1, u32::MAX] {
            let config = LogConfig {
                max_entry_bytes,
                ..LogConfig::default()
            };
            assert!(matches!(
                config.validate(),
                Err(ConfigSchemaError::ValidationError(_))
            ));
        }
    }
}
