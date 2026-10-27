//! Public responsiveness protocol; no diagnostic or readiness authority.

/// Explicit Canic method identity, independent of role and capability selection.
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0001;

/// A framed unit request carries no application input.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 64,
    allocation: 64,
    nodes: 8,
    depth: 4,
};

/// Bound the identity, role label, responsiveness and observation timestamp.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 256,
    depth: 16,
};

#[cfg(test)]
mod tests;
