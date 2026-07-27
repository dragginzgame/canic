//! Placement workflows for indexing, scaling, and sharding behavior.

pub mod acknowledgement;
pub mod allocation;
pub mod index;
pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
