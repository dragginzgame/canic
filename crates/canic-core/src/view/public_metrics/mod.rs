//! Module: view::public_metrics
//!
//! Responsibility: carry read-only history and performance projections from their owners.
//! Boundary: no collection, mutation, endpoint DTOs or query authorisation.

use crate::{model::public_metrics::PublicHistorySample, perf::PerfKey};

/// One exact series in chronological source-slot order, without storage bindings.
#[derive(Clone, Debug)]
pub struct PublicHistorySeries {
    pub unit: String,
    pub slots: Vec<PublicHistorySample>,
}

/// Full source measurement for bounded public projection, preserving quality.
pub struct PerfMeasurement {
    pub key: PerfKey,
    pub aggregate: ic_metrics::MeasurementAggregate,
}
