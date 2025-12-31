//! Lifecycle adapters.
//!
//! This module is public solely so it can be referenced by
//! macro expansions in downstream crates. It is not intended
//! for direct use.
//!
//! It must remain synchronous and minimal.

pub mod init;
pub mod upgrade;
