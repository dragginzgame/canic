//! Controller-owned state recovery; these updates remain usable when ordinary updates are fenced.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: crate::protocol::limits::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
    allocation: 262_144,
    nodes: 16_384,
    depth: 32,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 8_388_608,
    nodes: 262_144,
    depth: 64,
};
/// Change automatic funding and report bounded propagation outcomes.
pub mod funding {
    pub const METHOD: &str = "canic_root_set_cycles_funding";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0804;
}
/// Change Fleet status and report bounded propagation outcomes.
pub mod status {
    pub const METHOD: &str = "canic_root_set_fleet_status";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0805;
}
#[cfg(test)]
mod tests;
