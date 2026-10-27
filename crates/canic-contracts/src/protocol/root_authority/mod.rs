//! Controller-only Root authority observation, available during preparation.
pub const METHOD: &str = "canic_root_authority";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_2e00;
pub use crate::protocol::store_inventory::{REQUEST_LIMITS, RESPONSE_LIMITS};

#[cfg(test)]
mod tests;
