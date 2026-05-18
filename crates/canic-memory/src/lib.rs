//! Core stable-memory utilities shared across Canic consumers.
//!
//! This crate temporarily hosts the shared memory manager, eager TLS runtime,
//! and registry backend while Canic memory ownership moves into `canic-core`
//! and durable allocation-governance mechanics move into `ic-memory`.

/// Supported high-level API for bootstrapping, registering, and inspecting
/// stable-memory slots.
pub mod api;
mod ledger;
#[doc(hidden)]
pub mod manager;
mod policy;
/// Stable-memory range and ID registry used by the public API and macros.
pub mod registry;
#[doc(hidden)]
pub mod runtime;
pub use canic_cdk::serialize;
pub use canic_cdk::{impl_storable_bounded, impl_storable_unbounded};

pub use ::canic_cdk as cdk;

// internal derive support
#[doc(hidden)]
pub(crate) use thiserror::Error as ThisError;

// re-exports
#[doc(hidden)]
pub mod __reexports {
    pub use ctor;
}
