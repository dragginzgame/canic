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
    /// Scalar count; false exactness admits only a lower bound.
    Count { count: u128, exact: bool },
    /// Count and accumulated total from the same admitted observation cohort.
    /// Numeric caps remain exact unless their corresponding flag is true.
    Measurement {
        count: u64,
        total: u128,
        count_overflowed: bool,
        total_overflowed: bool,
    },
    /// Scalar quantity; false exactness admits only a lower bound.
    U128 { value: u128, exact: bool },
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
