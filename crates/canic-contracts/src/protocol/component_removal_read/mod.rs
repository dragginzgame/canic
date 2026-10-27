//! Controller-only read of retained top-level removal authority.
pub use crate::protocol::component_read::{REQUEST_LIMITS, RESPONSE_LIMITS};
pub const METHOD: &str = "canic_root_component_removal";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_3701;
#[cfg(test)]
mod tests;
