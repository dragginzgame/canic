//! Native-principal metric transport; aggregation stays with the metric owner.
use crate::dto::metrics::MetricValue;

/// One ordered metric row, preserving its optional principal dimension.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct MetricEntry {
    pub labels: Vec<String>,
    pub principal: Option<ic_platform::Principal>,
    pub value: MetricValue,
}
