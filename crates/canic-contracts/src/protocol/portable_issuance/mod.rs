//! Bounds for explicitly composed issuer endpoints; mounting stays application-owned.

pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 1_048_576,
    nodes: 16_384,
    depth: 16,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 131_072,
    allocation: 2_097_152,
    nodes: 65_536,
    depth: 32,
};
