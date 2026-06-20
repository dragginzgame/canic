//! Module: ops::blob_storage
//!
//! Responsibility: expose deterministic blob-storage conversions and state ops.
//! Does not own: endpoint authentication, workflow orchestration, or policy.
//! Boundary: workflow calls these ops after authorization and before storage effects.

pub mod conversion;
#[cfg(feature = "blob-storage-billing")]
pub mod funding;
pub mod lifecycle;
