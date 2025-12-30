use crate::dto::prelude::*;

///
/// IccMetricEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct IccMetricEntry {
    pub target: Principal,
    pub method: String,
    pub count: u64,
}
