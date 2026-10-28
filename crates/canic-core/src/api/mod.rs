//! Module: api
//!
//! Responsibility: public API facades for macro-generated canister endpoints.
//! Does not own: orchestration, business logic, policy, or storage invariants.
//! Boundary: maps endpoint calls into workflow calls and public errors.

pub mod application_checkpoint;
pub mod application_startup;
pub mod auth;
pub mod authority_restore;
pub mod call;
pub mod caller_authority;
pub mod cascade;
pub mod component_deployment;
pub mod component_runtime;
pub mod config;
pub mod error;
pub mod fixture_content;
pub mod fixture_provisioning;
pub mod fleet_activation;
pub mod fleet_admission_projection;
pub mod ic;
pub mod icp_refill;
pub mod intent;
pub mod lifecycle;
pub mod memory;
pub mod metadata;
pub mod observability;
pub mod placement;
pub mod public_status;
pub mod ready;
pub mod role;
pub mod rpc;
pub mod runtime;
pub mod state;
pub mod timer;
pub mod topology;

///
/// Read-only query re-exports
///
/// Only queries that satisfy ALL of the following may be re-exported directly:
///
/// - Read-only
/// - No orchestration or side effects
/// - No policy or invariant enforcement
/// - No internal `InternalError` in public signatures
/// - Return DTOs or primitives only
///
/// Queries that can fail with internal errors or enforce invariants
/// MUST be wrapped in an API façade instead.
///

pub mod cycles {
    pub use crate::workflow::runtime::cycles::query::CycleTrackerQuery;
}
pub mod env {
    pub use crate::workflow::env::query::EnvQuery;
}
pub mod icrc {
    pub use crate::workflow::icrc::query::{Icrc10Query, Icrc21Query};
}
pub mod log {
    pub use crate::workflow::log::query::LogQuery;
}
pub mod metrics {
    pub use crate::config::schema::MetricStorage;
    pub use crate::workflow::metrics::providers::{
        MetricDefinition, MetricKind, MetricObservation, ProviderMetricValue, ProviderMetricsError,
    };
    pub use crate::workflow::metrics::query::MetricsQuery;
    /// Native host composition; adds no endpoint, timer or implicit lifecycle.
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
}
