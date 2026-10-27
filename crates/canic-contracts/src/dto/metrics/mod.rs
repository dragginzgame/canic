use crate::dto::prelude::*;

pub use crate::values::metrics::MetricsKind;

//
// Metrics DTOs
//

//
// MetricEntry
//
// Unified metrics row.
//

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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

#[derive(Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct QueryPerfSample<T> {
    // Query result returned by the probe.
    pub value: T,

    // Local instruction counter observed in the same query call context.
    pub local_instructions: u64,
}
