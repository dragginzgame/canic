//! Placement workflows for binding, scaling, and sharding behavior.

pub mod acknowledgement;
pub mod allocation;
pub mod binding;
pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
