use serde::{Deserialize, Serialize};

///
/// TimerMetricEntry
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TimerMetricEntry {
    pub mode: String,
    pub delay_ms: u64,
    pub label: String,
    pub count: u64,
}
