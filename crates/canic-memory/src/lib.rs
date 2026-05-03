//! Core stable-memory utilities shared across Canic consumers.
//!
//! This crate hosts the shared memory manager, eager TLS helpers, registry
//! (ID/range reservation), and ergonomics macros (`ic_memory!`, `ic_memory_range!`,
//! `eager_static!`) so external crates can coordinate stable memory without
//! depending on the full `canic` stack.

/// Supported high-level API for bootstrapping, registering, and inspecting
/// stable-memory slots.
pub mod api;
mod macros;
#[doc(hidden)]
pub mod manager;
/// Stable-memory range and ID registry used by the public API and macros.
pub mod registry;
#[doc(hidden)]
pub mod runtime;
pub mod serialize;

pub use ::canic_cdk as cdk;

// internal derive support
#[doc(hidden)]
pub(crate) use thiserror::Error as ThisError;

// re-exports
#[doc(hidden)]
pub mod __reexports {
    pub use ctor;
}
