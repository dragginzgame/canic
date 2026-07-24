//! Public placement APIs grouped by placement strategy.

pub mod binding;
pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
