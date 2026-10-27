//! Controller-authenticated bounded automatic funding history.
pub const METHOD: &str = "canic_cycle_topups";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0102;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    allocation: 1024,
    nodes: 16,
    depth: 4,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1_048_576,
    allocation: 16_777_216,
    nodes: 262_144,
    depth: 24,
};
