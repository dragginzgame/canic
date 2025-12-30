//! Lifecycle adapters.
//!
//! This module is the **only** place that should be called directly
//! from IC lifecycle hooks (init / post_upgrade).
//!
//! It must remain synchronous and minimal.

pub mod init;
pub mod upgrade;
