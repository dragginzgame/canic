use crate::dto::prelude::*;

///
/// HttpMetricEntry
/// Snapshot entry pairing a method/label with its count.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HttpMetricEntry {
    pub method: String,
    pub label: String,
    pub count: u64,
}
