//! Controller-only typed binding observations.
pub const METHOD: &str = "canic_binding";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_010a;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 64,
    allocation: 512,
    nodes: 16,
    depth: 8,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 67_108_864,
    nodes: 4_194_304,
    depth: 64,
};
