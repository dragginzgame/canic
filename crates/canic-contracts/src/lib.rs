//! Canonical runtime-free Canic wire contracts and identifiers.
//!
//! This crate owns passive declarations and their deterministic codecs.
//! Storage, lifecycle, timers, authentication decisions, and effects remain
//! with the runtime and control-plane owners.

pub mod cycles;
pub mod deployment;
pub mod diagnostics;
pub mod dto;
pub mod ids;
pub mod log;
pub mod protocol;
pub mod serialization;
pub mod values;

/// Passive external dependencies needed by exported contract macros.
#[doc(hidden)]
pub mod __reexports {
    pub use candid;
    pub use serde;
}

#[cfg(test)]
mod test;
