use serde::{Deserialize, Serialize};

///
/// SystemMetricEntry
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SystemMetricEntry {
    pub kind: String,
    pub count: u64,
}
