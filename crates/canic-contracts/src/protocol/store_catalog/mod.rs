//! Typed protected Store observations; each method owns one response shape.
pub use crate::protocol::store_inventory::{REQUEST_LIMITS, RESPONSE_LIMITS};
/// Read retained fixture source progress.
pub mod fixture {
    pub const METHOD: &str = "canic_wasm_store_fixture_status";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1700;
}
/// Read the exact current grant, including a revoked revision.
pub mod grant {
    pub const METHOD: &str = "canic_wasm_store_fixture_grant";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1701;
}
/// Read manifest and per-chunk staging evidence.
pub mod template {
    pub const METHOD: &str = "canic_wasm_store_template_status";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1702;
}
#[cfg(test)]
mod tests;
