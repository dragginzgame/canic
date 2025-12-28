use candid::Principal;
use serde::{Deserialize, Serialize};

///
/// IccMetricEntry
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IccMetricEntry {
    pub target: Principal,
    pub method: String,
    pub count: u64,
}
