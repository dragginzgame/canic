use crate::dto::prelude::*;

pub use crate::domain::metrics::MetricsKind;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexported_metrics_kind_roundtrips_through_candid() {
        let kind = crate::domain::metrics::MetricsKind::Security;

        let bytes = candid::encode_one(kind).expect("encode metrics kind");
        let decoded: MetricsKind = candid::decode_one(&bytes).expect("decode metrics kind");

        assert!(matches!(decoded, MetricsKind::Security));
    }
}
