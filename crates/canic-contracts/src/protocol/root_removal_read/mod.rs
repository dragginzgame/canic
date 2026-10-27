//! Coordinator/controller observation of retained Root-removal progress.
pub use crate::protocol::component_read::{REQUEST_LIMITS, RESPONSE_LIMITS};
pub const METHOD: &str = "canic_root_removal";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_3702;
#[cfg(test)]
mod tests;
