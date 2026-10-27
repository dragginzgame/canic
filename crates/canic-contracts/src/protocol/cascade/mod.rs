//! Parent-authenticated cascade methods with exact shared role-independent state frames.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: crate::protocol::CASCADE_SNAPSHOT_MAX_BYTES,
    allocation: 262_144,
    nodes: 16_384,
    depth: 32,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: crate::protocol::CASCADE_SNAPSHOT_MAX_BYTES,
    allocation: 262_144,
    nodes: 16_384,
    depth: 32,
};
/// Parent-owned state propagation, also reachable while ordinary updates are stopped.
pub mod state {
    pub const METHOD: &str = "canic_synchronize_state";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0d00;
}
/// Parent-owned Wasm Store topology propagation.
pub mod topology {
    pub const METHOD: &str = "canic_synchronize_topology";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0d01;
}

#[cfg(test)]
mod tests;
