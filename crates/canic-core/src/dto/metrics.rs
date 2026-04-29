use crate::dto::prelude::*;

//
// Metrics DTOs
//

//
// MetricsKind
//
// Metric family selector.
//

#[derive(CandidType, Clone, Copy, Deserialize)]
pub enum MetricsKind {
    System,
    Icc,
    Http,
    Timer,
    Access,
    DelegatedAuth,
    RootCapability,
    CyclesFunding,
    Perf,
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
