//! Module: model::placement
//!
//! Responsibility: expose canonical placement values shared across layers.
//! Does not own: placement policy, storage access, or workflow orchestration.

pub mod allocation;
pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
