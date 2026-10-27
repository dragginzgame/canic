//! Shared non-root request-cycles capability transport; runtime owns authorization and replay.
pub const METHOD: &str = "canic_request_cycles";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0f00;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: crate::protocol::limits::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
    allocation: 65_536,
    nodes: 4096,
    depth: 16,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 8_388_608,
    nodes: 1_048_576,
    depth: 64,
};
#[cfg(test)]
mod tests;
