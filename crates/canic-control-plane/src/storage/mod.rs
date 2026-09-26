//! Persistent control-plane storage roots.

pub mod stable;
#[cfg(feature = "root-control-plane")]
pub mod transient;
