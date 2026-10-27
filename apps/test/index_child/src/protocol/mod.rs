//! Native admission probe contract for this installed application fixture.

pub const ADMISSION: u64 = 0x5445535400000300;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 128,
    depth: 16,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 262_144,
    nodes: 4096,
    depth: 32,
};

pub const STARTUP_CALLS: u64 = 0x5445_5354_0000_0612;
