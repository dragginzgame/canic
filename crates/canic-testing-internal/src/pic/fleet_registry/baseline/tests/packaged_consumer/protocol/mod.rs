//! Shared contract for the extracted-package application and its installed caller.

pub const METHOD: &str = "packaged_probe";
pub const METHOD_ID: u64 = 0x5445_5354_0000_0f00;
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    allocation: 16_384,
    nodes: 256,
    depth: 16,
};
