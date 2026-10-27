//! Controller query of exact child grant accounting.
pub const METHOD: &str = "canic_child_funding";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0107;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    allocation: 1024,
    nodes: 16,
    depth: 8,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    allocation: 65536,
    nodes: 1024,
    depth: 16,
};
