//! Bounded direct-child inventory; storage and pagination remain owned by topology.

/// One method identity across Root and child-provisioning roles.
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0005;
/// Two pagination integers within a bounded frame.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    allocation: 1024,
    nodes: 16,
    depth: 4,
};
/// Bounded child page, including principal, role, parent and installed module evidence.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1_048_576,
    allocation: 16_777_216,
    // Wire charges four visits per value, including each module-hash byte.
    nodes: 262_144,
    depth: 16,
};

#[cfg(test)]
mod tests;
