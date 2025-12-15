use serde::{Deserialize, Serialize};

///
/// Defaults
///

mod defaults {
    pub const fn max_entries() -> u64 {
        10_000
    }
}

///
/// LogConfig
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogConfig {
    #[serde(default = "defaults::max_entries")]
    pub max_entries: u64,

    #[serde(default)]
    pub max_age_secs: Option<u64>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            max_entries: defaults::max_entries(),
            max_age_secs: None,
        }
    }
}
