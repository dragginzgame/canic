//! Root-owned managed runtime configuration with exact operation receipts.
pub const METHOD: &str = "canic_configure_runtime";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0d02;
pub use crate::protocol::cascade::{REQUEST_LIMITS, RESPONSE_LIMITS};

#[cfg(test)]
mod tests;
