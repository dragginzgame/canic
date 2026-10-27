//! Bounded controller-only memory allocations diagnostics.
pub const METHOD: &str = "canic_memory_allocations";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0106;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 2_097_152,
    nodes: 65_536,
    depth: 16,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 67_108_864,
    nodes: 4_194_304,
    depth: 32,
};
