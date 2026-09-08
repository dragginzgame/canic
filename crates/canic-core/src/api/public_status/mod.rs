//! Module: api::public_status
//!
//! Responsibility: expose summary health, cached public reads, and trusted local sampling.
//! Does not own: publication authority, storage, or sampling schedules.
//! Boundary: publication methods are Rust APIs, never anonymous update endpoints.

use crate::{
    dto::{
        error::Error,
        public_status::{PublicHealth, PublicMetric, PublicMetricsRequest, PublicMetricsSnapshot},
    },
    workflow::metrics::publication::PublicMetricsWorkflow,
};

/// Public reads and explicit local snapshot publication for application update/timer owners.
pub struct PublicStatusApi;

impl PublicStatusApi {
    #[must_use]
    pub fn health() -> PublicHealth {
        PublicMetricsWorkflow::health()
    }
    #[must_use]
    pub fn metrics(request: PublicMetricsRequest) -> PublicMetricsSnapshot {
        PublicMetricsWorkflow::read(request)
    }
    pub fn sample_metrics() -> Result<(), Error> {
        PublicMetricsWorkflow::sample().map_err(Into::into)
    }
    pub fn record_application_metrics(metrics: Vec<PublicMetric>) -> Result<(), Error> {
        PublicMetricsWorkflow::record_application(metrics).map_err(Into::into)
    }
}
