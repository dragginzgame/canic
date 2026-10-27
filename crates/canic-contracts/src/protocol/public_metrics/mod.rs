//! Cached public metrics protocol; queries never trigger collection.

/// Explicit identity shared by every metrics-capable role.
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0002;

/// Family selector and bounded pagination inputs.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    allocation: 8192,
    nodes: 128,
    depth: 16,
};

/// Bound the existing public cache page, including native principal dimensions.
/// Allocation includes Wire's conservative repeated visitor/type charges.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 262_144,
    allocation: 16_777_216,
    nodes: 65_536,
    depth: 32,
};

#[cfg(test)]
mod tests;
