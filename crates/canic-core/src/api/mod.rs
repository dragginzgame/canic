//! Public API façade for canister endpoints.
//!
//! This module contains thin wrappers exposed to proc-macro–generated
//! endpoints. Functions here translate public API calls into internal
//! workflow or ops calls and map internal errors into `Error`.
//!
//! No orchestration or business logic should live here.
//! Any wrapper callable from an endpoint must return a `Result` so errors
//! are consistently mapped at the boundary.

pub mod access;
pub mod auth;
pub mod cascade;
pub mod config;
pub mod error;
pub mod ic;
pub mod icts;
pub mod lifecycle;
pub mod placement;
pub mod pool;
pub mod rpc;
pub mod state;
pub mod timer;
pub mod topology;
pub mod wasm;

///
/// Workflow Query Re-exports
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
pub mod memory {
    pub use crate::workflow::memory::query::MemoryQuery;
}
pub mod metrics {
    pub use crate::workflow::metrics::query::MetricsQuery;
}
