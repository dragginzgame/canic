//! Cached history protocol; reads never collect or interpolate observations.

/// Explicit identity shared by every history-capable role.
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0003;

/// Exact series selector, native principal and pagination.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1024,
    allocation: 16_384,
    nodes: 256,
    depth: 16,
};

/// Full retained window including counter deltas and timer lifetimes.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 262_144,
    allocation: 16_777_216,
    nodes: 65_536,
    depth: 32,
};
