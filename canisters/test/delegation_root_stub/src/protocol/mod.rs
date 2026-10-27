//! Native request contract for explicit proof provisioning in the Root fixture.

pub const PROVISION: u64 = 0x5445_5354_0000_0700;
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 128,
    depth: 16,
};
