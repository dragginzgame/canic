//! Delegation operation helpers layered atop `state::delegation`.
//!
//! The ops layer adds policy, logging, and cleanup around session caches and
//! registries while re-exporting the state-level helpers.

mod cache;
mod registry;

pub use cache::*;
pub use registry::*;
