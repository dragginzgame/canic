//! Caller-authorized component operation observations, including uncertain lifecycle evidence.
pub use crate::protocol::store_inventory::{REQUEST_LIMITS, RESPONSE_LIMITS};
pub mod child_allocation {
    pub const METHOD: &str = "canic_root_child_allocation";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3200;
}

/// Full Root provisioning progress and retained terminal receipts.
pub mod provisioning {
    pub const METHOD: &str = "canic_root_component_provisioning";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3201;
}

#[cfg(test)]
mod tests;

/// Complete Coordinator provisioning progress under the original operation authority.
pub mod coordinator_provisioning {
    pub const METHOD: &str = "canic_coordinator_component_provisioning";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3202;
}

/// Complete top-level allocation observation under retained operation authority.
pub mod allocation {
    pub const METHOD: &str = "canic_root_component_allocation";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3203;
}

/// Point-in-time Root membership under the original caller authorization.
pub mod membership {
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3204;
}
