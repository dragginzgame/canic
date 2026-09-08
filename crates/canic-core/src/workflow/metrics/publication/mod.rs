//! Module: workflow::metrics::publication
//!
//! Responsibility: sample explicitly selected families and expose public cache projections.
//! Does not own: authorization, timer registration, or snapshot records.
//! Boundary: trusted local updates sample; public queries only project cached values.

use crate::{
    InternalError,
    dto::public_status::{PublicHealth, PublicMetric, PublicMetricsRequest, PublicMetricsSnapshot},
    ops::{ic::IcOps, runtime::public_metrics::PublicMetricsOps},
};

/// Public snapshot reads and explicit update-side publication.
pub struct PublicMetricsWorkflow;

impl PublicMetricsWorkflow {
    #[must_use]
    pub fn health() -> PublicHealth {
        PublicMetricsOps::health()
    }
    #[must_use]
    pub fn read(request: PublicMetricsRequest) -> PublicMetricsSnapshot {
        PublicMetricsOps::read(request)
    }
    pub fn record_application(metrics: Vec<PublicMetric>) -> Result<(), InternalError> {
        PublicMetricsOps::record_application(metrics)
    }
    pub fn sample() -> Result<(), InternalError> {
        let families = PublicMetricsOps::enabled();
        if families.is_empty() {
            return Ok(());
        }
        let now = IcOps::now_nanos();
        for family in families {
            PublicMetricsOps::sample_family(family, now)?;
        }
        Ok(())
    }
}
