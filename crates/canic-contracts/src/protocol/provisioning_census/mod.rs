//! Bounded Root journal discovery; observing pages grants no effect authority.
pub const METHOD: &str = "canic_root_provisioning_release";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_1d00;
pub const REQUEST_LIMITS: ic_wire::Limits = crate::protocol::store_inventory::REQUEST_LIMITS;
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 262_144,
    allocation: 2_097_152,
    nodes: 262_144,
    depth: 64,
};

#[cfg(test)]
mod tests;
