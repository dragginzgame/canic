//! Controller-owned Store authority and exact operation reads.
pub use crate::protocol::store_inventory::{REQUEST_LIMITS, RESPONSE_LIMITS};
/// Full retained Store installation authority.
pub mod authority {
    pub const METHOD: &str = "canic_wasm_store_authority";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1900;
}
/// Activation evidence for the requested operation ID.
pub mod activation {
    pub const METHOD: &str = "canic_wasm_store_activation_status";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1901;
}
/// Garbage-collection evidence for the requested operation ID.
pub mod gc {
    pub const METHOD: &str = "canic_wasm_store_gc_status";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1902;
}
#[cfg(test)]
mod tests;
