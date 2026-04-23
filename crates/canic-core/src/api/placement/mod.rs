//! Public placement APIs grouped by placement strategy.

pub mod directory;
pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
