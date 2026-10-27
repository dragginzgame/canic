//! Typed registry mutation contracts; hashes name the unchanged canonical registry.
pub use crate::protocol::admission_control::{REQUEST_LIMITS, RESPONSE_LIMITS};
/// Join one exactly reviewed Root.
pub mod join {
    pub const METHOD: &str = "canic_coordinator_join_root";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2700;
}
/// Atomically activate the acknowledged Root set.
pub mod activate {
    pub const METHOD: &str = "canic_coordinator_activate_registry";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2701;
}

/// Synchronize one Root to the exact reviewed registry and Store bootstrap.
pub mod synchronize {
    pub const METHOD: &str = "canic_root_synchronize_registry";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2702;
}

/// Exact synchronization and mirror-activation receipt under the existing controller fence.
pub mod synchronization_status {
    pub const METHOD: &str = "canic_root_registry_synchronization";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3600;
}

#[cfg(test)]
mod tests;
