//! Canonical capability scope names for delegated auth.
//!
//! These constants are intentionally broad so apps/canisters can start with a
//! simple capability model and specialize later when needed.

pub const READ: &str = "read";
pub const WRITE: &str = "write";
pub const VERIFY: &str = "verify";
pub const ADMIN: &str = "admin";
