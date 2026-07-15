//! Module: model
//!
//! Responsibility: own authoritative runtime state models and storage invariants.
//! Does not own: stable storage access, orchestration, or platform side effects.
//! Boundary: ops accesses model state; persisted records and views are passive projections.

pub mod auth;
#[cfg(feature = "blob-storage")]
pub mod blob_storage;
pub mod cycles_funding;
pub mod env;
pub mod intent;
pub mod placement;
pub mod replay;
pub mod topology;
