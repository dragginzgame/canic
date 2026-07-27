//! Public placement APIs grouped by placement strategy.

pub mod index;
pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
