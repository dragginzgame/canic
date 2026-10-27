//! Separate native provisioning and synchronization methods; canonical storage remains unchanged.
pub use crate::protocol::admission_control::RESPONSE_LIMITS;
/// Preserve the existing accepted plan payload ceiling with a bounded native decoder.
pub const PLAN_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 8_388_608,
    allocation: 67_108_864,
    nodes: 2_097_152,
    depth: 64,
};
/// The exact Root batch carries fixed authority in addition to its canonical plan bytes.
pub const ROOT_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 8_454_144,
    ..PLAN_LIMITS
};
pub use crate::protocol::admission_control::REQUEST_LIMITS as SYNCHRONIZE_LIMITS;
/// Retain a complete controller-authorized plan before effects.
pub mod prepare {
    pub const METHOD: &str = "canic_coordinator_provision_components";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2a00;
}
/// Retain the exact Coordinator-selected Root batch.
pub mod accept {
    pub const METHOD: &str = "canic_root_provision_components";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2a01;
}
/// Advance the exact existing-member synchronization step.
pub mod synchronize {
    pub const METHOD: &str = "canic_root_synchronize_component_directories";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2a02;
}

#[cfg(test)]
mod tests;
