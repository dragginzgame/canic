//! Explicit synchronous metric-provider lifecycle and recording orchestration.
use crate::ops::runtime::metric_providers::{self as ops, host};
pub use ops::{
    MetricDefinition, MetricKind, MetricObservation, ProviderMetricValue, ProviderMetricsError,
};

pub fn register(
    provider: &'static str,
    definitions: &'static [MetricDefinition],
) -> Result<(), ProviderMetricsError> {
    host::register(provider, definitions)
}
pub fn activate(fresh: bool) -> Result<(), ProviderMetricsError> {
    host::activate(fresh)
}
pub fn add(provider: &str, name: &str, amount: u128) -> Result<(), ProviderMetricsError> {
    host::add(provider, name, amount)
}
pub fn snapshot(provider: &str) -> Result<Vec<MetricObservation>, ProviderMetricsError> {
    host::snapshot(provider)
}
pub fn memory_requests(
    authority: &crate::config::RoleRuntimeAuthority,
) -> Result<Vec<ic_memory::MemoryRequest>, ic_memory::MemoryDeclarationError> {
    host::role_memory_requests(authority)
}

pub fn record(provider: &str, name: &str, value: u64) -> Result<(), ProviderMetricsError> {
    host::record(provider, name, value)
}
