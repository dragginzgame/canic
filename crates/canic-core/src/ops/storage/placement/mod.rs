//! Module: ops::storage::placement
//!
//! Responsibility: group placement-related deterministic storage operations.
//! Does not own: placement policy, workflow orchestration, or endpoint DTOs.
//! Boundary: storage ops for directory, scaling, and sharding records.

pub mod directory;
pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
#[cfg(feature = "sharding")]
pub mod sharding_lifecycle;
