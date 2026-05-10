use crate::dto::prelude::*;

//
// Metrics DTOs
//

//
// MetricsKind
//
// Metric tier selector.
//

#[derive(CandidType, Clone, Copy, Deserialize)]
#[remain::sorted]
pub enum MetricsKind {
    Core,
    Placement,
    Platform,
    Runtime,
    Security,
    Storage,
}

//
// MetricEntry
//
// Unified metrics row.
//

#[derive(CandidType, Deserialize)]
pub struct MetricEntry {
    // Ordered labels.
    pub labels: Vec<String>,

    // Optional principal dimension.
    pub principal: Option<Principal>,

    // Metric payload.
    pub value: MetricValue,
}

//
// MetricValue
//

#[derive(CandidType, Deserialize)]
pub enum MetricValue {
    Count(u64),
    CountAndU64 { count: u64, value_u64: u64 },
    U128(u128),
}

//
// QueryPerfSample
//
// Same-call query performance sample.
//

#[derive(CandidType, Deserialize)]
pub struct QueryPerfSample<T> {
    // Query result returned by the probe.
    pub value: T,

    // Local instruction counter observed in the same query call context.
    pub local_instructions: u64,
}
