//! Native protocol shared by the release-binding probe and its installed caller.

pub const METHOD: &str = "binding";
pub const METHOD_ID: u64 = 0x5445_5354_0000_0e00;
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1024,
    allocation: 4096,
    nodes: 64,
    depth: 8,
};
