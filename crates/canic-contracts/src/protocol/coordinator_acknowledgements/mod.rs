//! Controller-only retained Root acknowledgement inventory.
/// Exact query method.
pub const METHOD: &str = "canic_coordinator_root_acknowledgements";
/// Native frame identity.
pub const METHOD_ID: u64 = 0x4341_4e49_0000_010e;
/// Unit request envelope.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 64,
    allocation: 512,
    nodes: 16,
    depth: 8,
};
/// Bounded inventory under the IC query response ceiling.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 8_388_608,
    nodes: 262_144,
    depth: 64,
};
#[cfg(test)]
mod tests;

/// Authenticated Root acknowledgement of its exact staged registry version.
pub mod acknowledge {
    pub use crate::protocol::admission_control::REQUEST_LIMITS;
    pub const METHOD: &str = "canic_coordinator_acknowledge_root_snapshot";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2200;
}
