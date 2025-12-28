use crate::{dto::prelude::*, model::metrics::access::AccessMetricKind};

///
/// AccessMetricEntry
/// Snapshot entry pairing an endpoint/stage with its count.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccessMetricEntry {
    pub endpoint: String,
    pub kind: AccessMetricKind,
    pub count: u64,
}
