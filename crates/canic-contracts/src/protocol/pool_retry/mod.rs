//! Fixed controller-owned pool recovery methods; existing pool workflows own paid effects.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: crate::protocol::limits::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
    allocation: 262_144,
    nodes: 16_384,
    depth: 32,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = REQUEST_LIMITS;
/// Retry a retained failed refill operation.
pub mod refill {
    pub const METHOD: &str = "canic_root_pool_retry_refill";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0c00;
}
/// Retry resetting one tracked pool canister.
pub mod reset {
    pub const METHOD: &str = "canic_root_pool_retry_reset";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0c01;
}

#[cfg(test)]
mod tests;
