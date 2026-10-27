//! Independent controller-only registry reads; exact prepared authority stays in workflow.
pub use crate::protocol::store_inventory::{REQUEST_LIMITS, RESPONSE_LIMITS};
/// Exact status observation.
pub mod status {
    pub const METHOD: &str = "canic_root_component_registry";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2f00;
}
/// Exact partition observation.
pub mod partition {
    pub const METHOD: &str = "canic_root_component_partition";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2f01;
}
/// Exact active partition observation.
pub mod active_partition {
    pub const METHOD: &str = "canic_root_component_active_partition";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2f02;
}

#[cfg(test)]
mod tests;
