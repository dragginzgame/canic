//! Module: api::public_status
//!
//! Responsibility: expose responsiveness, cached public reads, and trusted local sampling.
//! Does not own: publication authority, storage, or sampling schedules.
//! Boundary: publication methods are Rust APIs, never anonymous update endpoints.

use crate::{
    dto::{
        error::Error,
        public_status::{
            PublicHealth, PublicHistoryRequest, PublicHistorySnapshot, PublicMetric,
            PublicMetricsRequest, PublicMetricsSnapshot,
        },
    },
    workflow::metrics::publication::PublicMetricsWorkflow,
};

pub use crate::ops::runtime::public_metrics::ApplicationMetricsSampler;

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
    /// Cached chart history; missing slots are never filled by this query.
    #[must_use]
    pub fn history(request: PublicHistoryRequest) -> PublicHistorySnapshot {
        PublicMetricsWorkflow::history(request)
    }
    /// Set one bounded synchronous measurement provider in the application lifecycle hook.
    /// It must read maintained counters only, preserve source time/window identity, and
    /// return at most 256 aggregate rows. It runs only when Application is selected.
    pub fn set_application_sampler(sample: Option<ApplicationMetricsSampler>) {
        PublicMetricsWorkflow::set_application_sampler(sample);
    }
    pub fn sample_metrics() -> Result<(), Error> {
        PublicMetricsWorkflow::sample().map_err(Into::into)
    }
    pub fn record_application_metrics(metrics: Vec<PublicMetric>) -> Result<(), Error> {
        PublicMetricsWorkflow::record_application(metrics).map_err(Into::into)
    }
}
