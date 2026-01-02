use super::{ConfigSchemaError, Validate};
use serde::{Deserialize, Serialize};

///
/// Defaults
///

mod defaults {
    pub const fn max_entries() -> u64 {
        10_000
    }

    pub const fn max_entry_bytes() -> u32 {
        16_384
    }
}

pub const MAX_LOG_ENTRIES: u64 = 100_000;

///
/// LogConfig
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_field_names)]
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

impl Validate for LogConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        if self.max_entries > MAX_LOG_ENTRIES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "log.max_entries {} exceeds max {}",
                self.max_entries, MAX_LOG_ENTRIES
            )));
        }

        Ok(())
    }
}
