//! Manual sandbox method identities and bounded native frames.

pub const PING: u64 = 0x5342_4f58_0000_0001;
pub const ECHO: u64 = 0x5342_4f58_0000_0002;

// The former 32-byte echo admission ceiling now bounds the complete Wire frame.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 32,
    allocation: 128,
    nodes: 8,
    depth: 4,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 262_144,
    nodes: 4096,
    depth: 32,
};
