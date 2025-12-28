use crate::dto::prelude::*;

///
/// SystemMetricEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SystemMetricEntry {
    pub kind: String,
    pub count: u64,
}
