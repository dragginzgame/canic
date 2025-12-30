//! Core stable-memory utilities shared across Canic consumers.
//!
//! This crate hosts the shared memory manager, eager TLS helpers, registry
//! (ID/range reservation), and ergonomics macros (`ic_memory!`, `ic_memory_range!`,
//! `eager_static!`) so external crates can coordinate stable memory without
//! depending on the full `canic` stack.

pub mod macros;
pub mod manager;
pub mod registry;
pub mod runtime;
pub mod serialize;

pub use ::canic_cdk as cdk;

// internal types
pub use manager::MEMORY_MANAGER;
pub use runtime::init_eager_tls;
pub use thiserror::Error as ThisError;

// re-exports
#[doc(hidden)]
pub mod __reexports {
    pub use ctor;
}
