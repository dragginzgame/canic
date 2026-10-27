//! Controller health diagnostics, distinct from public responsiveness.
pub const METHOD: &str = "canic_health";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0103;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 64,
    allocation: 512,
    nodes: 16,
    depth: 8,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1_048_576,
    allocation: 16_777_216,
    nodes: 262_144,
    depth: 24,
};
