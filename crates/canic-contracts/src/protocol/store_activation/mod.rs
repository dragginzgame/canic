//! Store credential preparation and activation retain their original operation authority.
pub use crate::protocol::admission_control::{REQUEST_LIMITS, RESPONSE_LIMITS};
/// Retain the exact credential generation before activation.
pub mod prepare {
    pub const METHOD: &str = "canic_wasm_store_prepare_credential";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1500;
}
/// Activate the retained generation and exact cascade evidence.
pub mod activate {
    pub const METHOD: &str = "canic_wasm_store_activate";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1501;
}

#[cfg(test)]
mod tests;
