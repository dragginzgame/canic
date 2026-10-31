//! Module: api::metrics
//!
//! Responsibility: expose metric queries and explicit provider operations.
//! Does not own: registration policy, arithmetic, storage or lifecycle orchestration.
//! Boundary: public facade delegates to the owning workflows.

pub use crate::config::schema::MetricStorage;
pub use crate::workflow::metrics::providers::{
    MetricDefinition, MetricKind, MetricObservation, ProviderMetricValue, ProviderMetricsError,
};
pub use crate::workflow::metrics::query::MetricsQuery;

///
/// MetricProviderApi
///
/// Explicit host composition; adds no endpoint, timer or implicit lifecycle.
///

pub struct MetricProviderApi;
impl MetricProviderApi {
    /// Record one sample, including a real zero, for a declared measurement.
    /// Returns a typed lifecycle, declaration or storage refusal.
    pub fn record(provider: &str, name: &str, value: u64) -> Result<(), ProviderMetricsError> {
        crate::workflow::metrics::providers::record(provider, name, value)
    }
    /// Register a fixed roster in the synchronous init/restore participant.
    /// Refuses duplicate/invalid declarations, capacity overflow or late use.
    pub fn register(
        provider: &'static str,
        definitions: &'static [MetricDefinition],
    ) -> Result<(), ProviderMetricsError> {
        crate::workflow::metrics::providers::register(provider, definitions)
    }
    #[doc(hidden)]
    pub fn activate(fresh: bool) -> Result<(), ProviderMetricsError> {
        crate::workflow::metrics::providers::activate(fresh)
    }
    /// Add a diagnostic scalar, preserving sticky overflow quality.
    /// Refuses undeclared counters and unavailable retained storage.
    pub fn add(provider: &str, name: &str, amount: u128) -> Result<(), ProviderMetricsError> {
        crate::workflow::metrics::providers::add(provider, name, amount)
    }
    /// Read declared observations without writes, sampling or remote calls.
    /// Missing initialization/storage is an error, never an invented zero.
    pub fn snapshot(provider: &str) -> Result<Vec<MetricObservation>, ProviderMetricsError> {
        crate::workflow::metrics::providers::snapshot(provider)
    }
    #[doc(hidden)]
    pub fn memory_requests(
        authority: &crate::config::RoleRuntimeAuthority,
    ) -> Result<Vec<ic_memory::MemoryRequest>, ic_memory::MemoryDeclarationError> {
        crate::workflow::metrics::providers::memory_requests(authority)
    }
}
