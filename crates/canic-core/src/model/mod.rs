//! Module: model
//!
//! Responsibility: own authoritative runtime state models and storage invariants.
//! Does not own: stable storage access, orchestration, or platform side effects.
//! Boundary: ops accesses model state; persisted records and views are passive projections.

#[cfg(feature = "blob-storage")]
pub mod blob_storage;
pub mod intent;
pub mod replay;
