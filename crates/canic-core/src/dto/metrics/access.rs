use crate::dto::prelude::*;

///
/// AccessMetricKind
/// Enumerates the access-control stage that rejected the call.
///

#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[remain::sorted]
pub enum AccessMetricKind {
    Auth,
    Guard,
    Rule,
}

///
/// AccessMetricEntry
/// Snapshot entry pairing an endpoint/stage with its count.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AccessMetricEntry {
    pub endpoint: String,
    pub kind: AccessMetricKind,
    pub count: u64,
}
