//! Controller-only observation of exact subtree-removal evidence.
pub use crate::protocol::component_read::{REQUEST_LIMITS, RESPONSE_LIMITS};
pub const METHOD: &str = "canic_root_subtree_removal";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_3700;
#[cfg(test)]
mod tests;
