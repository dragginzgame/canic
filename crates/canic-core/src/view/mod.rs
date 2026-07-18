//! Module: view
//!
//! Responsibility: group internal read-only projections over stored or runtime state.
//! Does not own: endpoint DTOs, stable records, or workflow decisions.
//! Boundary: ops and workflow use views internally before endpoint DTO shaping.

#[cfg(feature = "blob-storage-billing")]
pub mod blob_storage;
pub mod icp_refill;
pub mod intent;
pub mod pool;
pub mod topology;
