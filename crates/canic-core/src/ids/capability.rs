//! Module: ids::capability
//! Responsibility: canonical capability scope names for delegated auth.
//! Does not own: authorization policy or capability validation.
//! Boundary: exposes broad string constants that consumers may specialize later.

pub const READ: &str = "read";
pub const WRITE: &str = "write";
pub const VERIFY: &str = "verify";
pub const ADMIN: &str = "admin";
