//! Module: api
//!
//! Responsibility: public API facades for macro-generated canister endpoints.
//! Does not own: orchestration, business logic, policy, or storage invariants.
//! Boundary: maps endpoint calls into workflow calls and public errors.

pub mod auth;
#[cfg(feature = "blob-storage")]
pub mod blob_storage;
pub mod cascade;
pub mod config;
pub mod error;
pub mod ic;
pub mod icp_refill;
pub mod intent;
pub mod lifecycle;
pub mod memory;
pub mod metadata;
pub mod placement;
pub mod pool;
pub mod ready;
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
    pub use crate::workflow::metrics::query::MetricsQuery;
}
