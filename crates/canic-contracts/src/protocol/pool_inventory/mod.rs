//! Controller-only bounded physical inventory page.
pub const METHOD: &str = "canic_root_pool";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_3000;
pub use crate::protocol::store_inventory::{REQUEST_LIMITS, RESPONSE_LIMITS};

#[cfg(test)]
mod tests;
