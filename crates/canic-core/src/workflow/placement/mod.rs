//! Placement workflows for directory, scaling, and sharding behavior.

pub mod directory;
pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
