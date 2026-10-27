//! Exact Store grant mutations and cycle-safe retirement controls.
pub use crate::protocol::store_publication::{REQUEST_LIMITS, RESPONSE_LIMITS};
/// Compare-and-set the exact target's fixture read grant.
pub mod grant {
    pub const METHOD: &str = "canic_wasm_store_set_fixture_grant";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1600;
}
/// Transfer only reclaimable cycles to the existing controlled destination.
pub mod reclaim {
    pub const METHOD: &str = "canic_wasm_store_reclaim_deletion_cycles";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1601;
}
#[cfg(test)]
mod tests;
