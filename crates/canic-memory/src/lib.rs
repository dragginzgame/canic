//! Core stable-memory utilities shared across Canic consumers.
//!
//! This crate hosts the shared memory manager, eager TLS helpers, registry
//! (ID/range reservation), and ergonomics macros (`ic_memory!`, `ic_memory_range!`,
//! `eager_static!`) so external crates can coordinate stable memory without
//! depending on the full `canic` stack.

pub mod macros;
pub mod manager;
pub mod ops;
pub mod registry;
pub mod runtime;

// export cdk
pub use ::canic_cdk as cdk;

// internal types
pub use manager::MEMORY_MANAGER;
pub use registry::{
    MemoryRange, MemoryRegistry, MemoryRegistryEntry, MemoryRegistryError, MemoryRegistryView,
    drain_pending_ranges, drain_pending_registrations,
};
pub use runtime::init_eager_tls;

pub mod export {
    pub use ctor;
}
