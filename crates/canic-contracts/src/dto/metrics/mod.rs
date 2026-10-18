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

#[derive(CandidType, Debug, Deserialize)]
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

#[derive(CandidType, Debug, Deserialize)]
pub enum MetricValue {
    Count(u64),
    /// Count and accumulated total from the same admitted observation cohort.
    /// Numeric caps remain exact unless their corresponding flag is true.
    Measurement {
        count: u64,
        total: u128,
        count_overflowed: bool,
        total_overflowed: bool,
    },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexported_metrics_kind_roundtrips_through_candid() {
        let kind = crate::values::metrics::MetricsKind::Security;

        let bytes = candid::encode_one(kind).expect("encode metrics kind");
        let decoded: MetricsKind = candid::decode_one(&bytes).expect("decode metrics kind");

        assert!(matches!(decoded, MetricsKind::Security));
    }
}
